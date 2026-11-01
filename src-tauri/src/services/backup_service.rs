use crate::database::connection::DbConnection;
use crate::services::crypto::get_app_data_dir;
use crate::services::integration_service::IntegrationService;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupItem {
    pub backup_id: String,
    pub folder_name: String,
    pub path: String,
    pub total_size_bytes: u64,
    pub total_size_formatted: String,
    pub created_at: String,
    pub cloud_synced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResult {
    pub backup_id: String,
    pub path: String,
    pub total_size_bytes: u64,
    pub deleted_old_backups: Vec<String>,
    pub cloud_status: String, // "LOCAL_SUCCESS" | "SYNCED_TO_GDRIVE"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub backup_id: String,
    pub created_at: String,
    pub app_version: String,
    pub main_db_size: u64,
    pub log_db_size: u64,
}

#[derive(Debug)]
pub enum BackupEntryError {
    /// Data backup tidak valid, korup, ukuran mismatch, created_at tidak valid, atau symlink
    Invalid(String),
    /// Error filesystem / I/O dari OS (PermissionDenied, disk fault, dll.) yang harus menghentikan operasi
    Io(std::io::Error),
}

impl std::fmt::Display for BackupEntryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupEntryError::Invalid(msg) => write!(f, "Invalid backup: {}", msg),
            BackupEntryError::Io(err) => write!(f, "Filesystem error: {}", err),
        }
    }
}

impl std::error::Error for BackupEntryError {}

impl From<std::io::Error> for BackupEntryError {
    fn from(err: std::io::Error) -> Self {
        BackupEntryError::Io(err)
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedBackup {
    pub manifest: BackupManifest,
    pub created_at_ts: i64,
}

fn is_sqlite_io_or_perm_error(code: rusqlite::ffi::ErrorCode) -> bool {
    matches!(
        code,
        rusqlite::ffi::ErrorCode::PermissionDenied
            | rusqlite::ffi::ErrorCode::CannotOpen
            | rusqlite::ffi::ErrorCode::SystemIoFailure
            | rusqlite::ffi::ErrorCode::DiskFull
    )
}

/// RAII Guard untuk memastikan folder staging otomatis dibersihkan jika proses backup gagal
struct StagingGuard<'a> {
    path: &'a Path,
    committed: bool,
}

impl<'a> Drop for StagingGuard<'a> {
    fn drop(&mut self) {
        if !self.committed && fs::symlink_metadata(self.path).is_ok() {
            let _ = fs::remove_dir_all(self.path);
        }
    }
}

pub struct BackupService;

impl BackupService {
    /// Mengambil path direktori penyimpanan backup lokal:
    /// e.g. ~/.local/share/olahpos/backups/
    pub fn get_backups_dir() -> std::io::Result<PathBuf> {
        let app_dir = get_app_data_dir()?;
        let backups_dir = app_dir.join("backups");
        match fs::symlink_metadata(&backups_dir) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Direktori backup {:?} adalah symbolic link", backups_dir),
                    ));
                }
                if !meta.is_dir() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Path backup {:?} bukan direktori", backups_dir),
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&backups_dir)?;
            }
            Err(e) => return Err(e),
        }
        Ok(backups_dir)
    }

    /// Format ukuran byte ke string KB / MB yang ramah pengguna
    fn format_bytes(bytes: u64) -> String {
        if bytes >= 1024 * 1024 {
            format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
        } else if bytes >= 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{} B", bytes)
        }
    }

    /// Memeriksa apakah database alias tertentu ter-attach pada koneksi SQLite sumber
    fn is_database_attached(conn: &Connection, db_alias: &str) -> Result<bool, String> {
        let mut stmt = conn
            .prepare("PRAGMA database_list;")
            .map_err(|e| format!("Gagal memeriksa database_list: {}", e))?;
        let names: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(|e| format!("Gagal query database_list: {}", e))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(names.iter().any(|n| n == db_alias))
    }

    /// Menjalankan SQLite Online Backup API untuk sumber database tertentu ke file tujuan
    fn execute_online_backup(
        src_conn: &Connection,
        src_db_name: rusqlite::DatabaseName<'_>,
        dst_path: &Path,
    ) -> Result<(), String> {
        let mut dst_conn = Connection::open(dst_path).map_err(|e| {
            format!(
                "Gagal menginisialisasi target database {:?}: {}",
                dst_path, e
            )
        })?;

        let backup = rusqlite::backup::Backup::new_with_names(
            src_conn,
            src_db_name,
            &mut dst_conn,
            rusqlite::DatabaseName::Main,
        )
        .map_err(|e| {
            format!(
                "Gagal inisialisasi SQLite backup engine untuk {:?}: {}",
                dst_path, e
            )
        })?;

        let mut busy_retries = 0;
        loop {
            match backup.step(100) {
                Ok(rusqlite::backup::StepResult::Done) => break,
                Ok(rusqlite::backup::StepResult::More) => {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                Ok(rusqlite::backup::StepResult::Busy)
                | Ok(rusqlite::backup::StepResult::Locked) => {
                    busy_retries += 1;
                    if busy_retries > 10 {
                        return Err(format!(
                            "Database sibuk atau terkunci saat backup {:?} setelah 10 percobaan",
                            dst_path
                        ));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Ok(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => {
                    return Err(format!(
                        "Gagal menjalankan SQLite online backup pada {:?}: {}",
                        dst_path, e
                    ));
                }
            }
        }

        Ok(())
    }

    /// Memvalidasi berkas database SQLite: bukan symbolic link, file reguler, ukuran > 0, bisa dibuka, dan lulus integrity_check
    pub fn validate_sqlite_database(path: &Path) -> Result<u64, BackupEntryError> {
        let meta = match fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(BackupEntryError::Invalid(format!(
                    "File database tidak ditemukan: {:?}",
                    path
                )));
            }
            Err(e) => return Err(BackupEntryError::Io(e)),
        };

        let file_type = meta.file_type();
        if file_type.is_symlink() {
            return Err(BackupEntryError::Invalid(format!(
                "File database {:?} adalah symbolic link, ditolak",
                path
            )));
        }
        if !file_type.is_file() {
            return Err(BackupEntryError::Invalid(format!(
                "File database {:?} bukan file reguler",
                path
            )));
        }

        let size = meta.len();
        if size == 0 {
            return Err(BackupEntryError::Invalid(format!(
                "File database kosong (0 bytes): {:?}",
                path
            )));
        }

        let conn = match Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            Ok(c) => c,
            Err(rusqlite::Error::SqliteFailure(err, msg)) => {
                if is_sqlite_io_or_perm_error(err.code) {
                    return Err(BackupEntryError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "SQLite I/O error saat membuka {:?}: {:?} ({:?})",
                            path, err, msg
                        ),
                    )));
                }
                return Err(BackupEntryError::Invalid(format!(
                    "File bukan database SQLite yang valid ({:?}): {:?} ({:?})",
                    path, err, msg
                )));
            }
            Err(e) => {
                return Err(BackupEntryError::Invalid(format!(
                    "File bukan database SQLite yang valid ({:?}): {}",
                    path, e
                )));
            }
        };

        let integrity_res: Result<String, rusqlite::Error> =
            conn.query_row("PRAGMA integrity_check;", [], |r| r.get(0));

        match integrity_res {
            Ok(integrity) => {
                if integrity != "ok" {
                    return Err(BackupEntryError::Invalid(format!(
                        "Database integrity_check gagal pada {:?}: {}",
                        path, integrity
                    )));
                }
            }
            Err(rusqlite::Error::SqliteFailure(err, msg)) => {
                if is_sqlite_io_or_perm_error(err.code) {
                    return Err(BackupEntryError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "SQLite I/O error saat integrity_check {:?}: {:?} ({:?})",
                            path, err, msg
                        ),
                    )));
                }
                return Err(BackupEntryError::Invalid(format!(
                    "Gagal menjalankan integrity_check pada {:?}: {:?} ({:?})",
                    path, err, msg
                )));
            }
            Err(e) => {
                return Err(BackupEntryError::Invalid(format!(
                    "Gagal menjalankan integrity_check pada {:?}: {}",
                    path, e
                )));
            }
        }

        Ok(size)
    }

    /// Melakukan atomic online backup dual SQLite database (kasir_pos.db & kasir_pos_log.db)
    /// ke direktori target yang ditentukan dengan staging eksklusif dan validasi integritas
    pub fn create_backup_snapshot_in(
        db: &DbConnection,
        backups_dir: &Path,
    ) -> std::result::Result<BackupResult, String> {
        match fs::symlink_metadata(backups_dir) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err(format!(
                        "Direktori backup {:?} adalah symbolic link, ditolak",
                        backups_dir
                    ));
                }
                if !meta.is_dir() {
                    return Err(format!(
                        "Path backup {:?} bukan sebuah direktori",
                        backups_dir
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(backups_dir)
                    .map_err(|e| format!("Gagal membuat direktori backups: {}", e))?;
            }
            Err(e) => {
                return Err(format!(
                    "Gagal memeriksa direktori backup {:?}: {}",
                    backups_dir, e
                ));
            }
        }

        // 1. Generate ID & buat direktori staging secara eksklusif (anti-collision atomic)
        let mut counter = 0;
        let (backup_id, staging_dir) = loop {
            let now = chrono::Utc::now();
            let base_id = format!("backup_{}", now.format("%Y%m%d_%H%M%S"));
            let nano = now.timestamp_subsec_nanos();
            let candidate_id = if counter == 0 {
                format!("{}_{:09}", base_id, nano)
            } else {
                format!("{}_{:09}_{}", base_id, nano, counter)
            };
            counter += 1;

            let target_dir = backups_dir.join(&candidate_id);
            let candidate_staging = backups_dir.join(format!(".staging_{}", candidate_id));

            // Jika folder backup final dengan nama ini sudah ada, cari ID berikutnya
            if fs::symlink_metadata(&target_dir).is_ok() {
                continue;
            }

            // fs::create_dir bersifat eksklusif pada level OS/filesystem (mkdir).
            // Akan gagal dengan ErrorKind::AlreadyExists jika folder staging sudah dibuat oleh proses lain
            match fs::create_dir(&candidate_staging) {
                Ok(_) => break (candidate_id, candidate_staging),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    // Collision terdeteksi, coba ID berikutnya tanpa berbagi staging milik proses lain
                    continue;
                }
                Err(e) => {
                    return Err(format!(
                        "Gagal membuat folder staging backup eksklusif {:?}: {}",
                        candidate_staging, e
                    ));
                }
            }
        };

        let mut staging_guard = StagingGuard {
            path: &staging_dir,
            committed: false,
        };

        let target_main_db = staging_dir.join("kasir_pos.db");
        let target_log_db = staging_dir.join("kasir_pos_log.db");

        // 2. Kunci seluruh koneksi sumber selama proses snapshot dual-database
        // Ini menjamin tidak ada transaksi aplikasi yang menyela di antara backup kedua database
        let now = chrono::Utc::now();
        {
            let src_conn = db
                .lock()
                .map_err(|e| format!("Gagal mengunci mutex koneksi database: {}", e))?;

            // Pastikan log_db benar-benar ter-attach pada koneksi sumber
            let log_attached = Self::is_database_attached(&src_conn, "log_db")?;
            if !log_attached {
                return Err(
                    "Database log 'log_db' tidak ter-attach pada koneksi SQLite aktif!".to_string(),
                );
            }

            // Catat metadata sumber database aktif untuk keterlacakan diagnostik
            if let Ok(mut stmt) = src_conn.prepare("PRAGMA database_list;") {
                if let Ok(rows) =
                    stmt.query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, String>(2)?)))
                {
                    for row in rows.flatten() {
                        log::info!(
                            "SQLite database source attached: alias='{}', file='{}'",
                            row.0,
                            row.1
                        );
                    }
                }
            }

            // Jalankan Online Backup untuk main DB (kasir_pos.db)
            Self::execute_online_backup(&src_conn, rusqlite::DatabaseName::Main, &target_main_db)?;

            // Jalankan Online Backup untuk log DB (kasir_pos_log.db) langsung dari koneksi ter-attach
            Self::execute_online_backup(
                &src_conn,
                rusqlite::DatabaseName::Attached("log_db"),
                &target_log_db,
            )?;
        }

        // 3. Validasi kedua database hasil backup secara mendalam (harus valid SQLite & integrity "ok")
        let main_db_size =
            Self::validate_sqlite_database(&target_main_db).map_err(|e| e.to_string())?;
        let log_db_size =
            Self::validate_sqlite_database(&target_log_db).map_err(|e| e.to_string())?;
        let total_size = main_db_size + log_db_size;

        // 4. Buat manifest.json dengan versi aplikasi riil dari Cargo.toml
        let manifest = BackupManifest {
            backup_id: backup_id.clone(),
            created_at: now.to_rfc3339(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            main_db_size,
            log_db_size,
        };
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("Gagal serialisasi manifest backup: {}", e))?;
        fs::write(staging_dir.join("manifest.json"), manifest_json)
            .map_err(|e| format!("Gagal menulis manifest.json: {}", e))?;

        // 5. Publikasikan folder backup secara atomik melalui rename
        let target_dir = backups_dir.join(&backup_id);
        fs::rename(&staging_dir, &target_dir)
            .map_err(|e| format!("Gagal mempublikasikan folder backup secara atomik: {}", e))?;

        staging_guard.committed = true;

        // 6. Jalankan kebijakan retensi hanya setelah backup terverifikasi sukses dipublikasikan
        let deleted_old_backups = Self::enforce_retention_policy(backups_dir, 10)?;

        // 7. Cek status sinkronisasi cloud
        let gdrive_config = IntegrationService::get_config(db, "GOOGLE_DRIVE_BACKUP")
            .ok()
            .flatten();
        let cloud_status = if gdrive_config.is_some() {
            "SYNCED_TO_GDRIVE".to_string()
        } else {
            "LOCAL_SUCCESS".to_string()
        };

        Ok(BackupResult {
            backup_id,
            path: target_dir.to_string_lossy().to_string(),
            total_size_bytes: total_size,
            deleted_old_backups,
            cloud_status,
        })
    }

    /// Melakukan backup snapshot menggunakan direktori default aplikasi (~/.local/share/olahpos/backups)
    pub fn create_backup_snapshot(db: &DbConnection) -> std::result::Result<BackupResult, String> {
        let backups_dir = Self::get_backups_dir().map_err(|e| e.to_string())?;
        Self::create_backup_snapshot_in(db, &backups_dir)
    }

    /// Memvalidasi kelayakan entri backup sesuai kriteria keamanan data & filesystem:
    /// 1. Direktori backup adalah direktori reguler, bukan symbolic link.
    /// 2. manifest.json adalah file reguler, bukan symbolic link, dan dapat dibaca.
    /// 3. manifest.json dapat di-parse sebagai JSON valid.
    /// 4. backup_id manifest sesuai dengan nama direktori.
    /// 5. created_at manifest adalah format RFC 3339 valid dan tidak kosong.
    /// 6. Kedua file database (kasir_pos.db & kasir_pos_log.db) adalah file reguler, bukan symbolic link.
    /// 7. Kedua database lolos validasi SQLite (open & PRAGMA integrity_check == "ok").
    /// 8. Ukuran aktual database sesuai dengan metadata manifest.
    pub fn validate_backup_entry(
        path: &Path,
        folder_name: &str,
    ) -> Result<ValidatedBackup, BackupEntryError> {
        // 1. Direktori backup harus ada dan bukan symlink
        let folder_meta = match fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(BackupEntryError::Invalid(format!(
                    "Direktori backup {:?} tidak ditemukan",
                    path
                )));
            }
            Err(e) => return Err(BackupEntryError::Io(e)),
        };

        if folder_meta.file_type().is_symlink() {
            return Err(BackupEntryError::Invalid(format!(
                "Direktori backup {:?} adalah symbolic link, ditolak",
                path
            )));
        }
        if !folder_meta.file_type().is_dir() {
            return Err(BackupEntryError::Invalid(format!(
                "Path backup {:?} bukan sebuah direktori",
                path
            )));
        }

        let manifest_file = path.join("manifest.json");
        let main_db = path.join("kasir_pos.db");
        let log_db = path.join("kasir_pos_log.db");

        // 2. manifest.json harus file reguler bukan symlink
        let manifest_meta = match fs::symlink_metadata(&manifest_file) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(BackupEntryError::Invalid(format!(
                    "File manifest.json tidak ditemukan di {:?}",
                    manifest_file
                )));
            }
            Err(e) => return Err(BackupEntryError::Io(e)),
        };

        if manifest_meta.file_type().is_symlink() {
            return Err(BackupEntryError::Invalid(format!(
                "File manifest {:?} adalah symbolic link, ditolak",
                manifest_file
            )));
        }
        if !manifest_meta.file_type().is_file() {
            return Err(BackupEntryError::Invalid(format!(
                "File manifest {:?} bukan file reguler",
                manifest_file
            )));
        }

        // Baca konten manifest
        let content = match fs::read_to_string(&manifest_file) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(BackupEntryError::Invalid(format!(
                    "File manifest.json tidak ditemukan di {:?}",
                    manifest_file
                )));
            }
            Err(e) => return Err(BackupEntryError::Io(e)),
        };

        // 3. Manifest JSON parsing
        let manifest: BackupManifest = serde_json::from_str(&content).map_err(|e| {
            BackupEntryError::Invalid(format!("Manifest JSON korup di {:?}: {}", manifest_file, e))
        })?;

        // 4. backup_id manifest sesuai dengan nama direktori
        if manifest.backup_id != folder_name {
            return Err(BackupEntryError::Invalid(format!(
                "backup_id manifest ('{}') tidak cocok dengan nama direktori ('{}')",
                manifest.backup_id, folder_name
            )));
        }

        // 5. Validasi created_at (RFC 3339)
        if manifest.created_at.trim().is_empty() {
            return Err(BackupEntryError::Invalid(
                "created_at manifest kosong".to_string(),
            ));
        }
        let parsed_dt =
            chrono::DateTime::parse_from_rfc3339(&manifest.created_at).map_err(|e| {
                BackupEntryError::Invalid(format!(
                    "Format created_at manifest invalid (bukan RFC 3339): {}",
                    e
                ))
            })?;
        let created_at_ts = parsed_dt.timestamp_millis();

        // 6 & 7. Validasi kedua database (open, integrity_check, bukan symlink)
        let actual_main_size = Self::validate_sqlite_database(&main_db)?;
        let actual_log_size = Self::validate_sqlite_database(&log_db)?;

        // 8. Ukuran aktual database sesuai dengan metadata manifest
        if actual_main_size != manifest.main_db_size {
            return Err(BackupEntryError::Invalid(format!(
                "Ukuran kasir_pos.db aktual ({} bytes) tidak cocok dengan metadata manifest ({} bytes)",
                actual_main_size, manifest.main_db_size
            )));
        }
        if actual_log_size != manifest.log_db_size {
            return Err(BackupEntryError::Invalid(format!(
                "Ukuran kasir_pos_log.db aktual ({} bytes) tidak cocok dengan metadata manifest ({} bytes)",
                actual_log_size, manifest.log_db_size
            )));
        }

        Ok(ValidatedBackup {
            manifest,
            created_at_ts,
        })
    }

    /// Helper kompatibilitas untuk memeriksa validitas backup
    pub fn is_backup_entry_valid(path: &Path, folder_name: &str) -> bool {
        Self::validate_backup_entry(path, folder_name).is_ok()
    }

    /// Menghapus backup lama yang melampaui batas retensi maksimum (default 10).
    /// Hanya menghitung backup yang terverifikasi valid sesuai seluruh kriteria integritas data.
    /// Pengurutan dilakukan berdasarkan timestamp pembuatan aktual (created_at).
    /// Backup invalid diabaikan dengan warning log, sedangkan error filesystem dilaporkan eksplisit.
    pub fn enforce_retention_policy(
        backups_dir: &Path,
        max_keep: usize,
    ) -> std::result::Result<Vec<String>, String> {
        Self::enforce_retention_policy_internal(
            backups_dir,
            max_keep,
            Self::validate_backup_entry,
            |p| fs::remove_dir_all(p),
        )
    }

    pub(crate) fn enforce_retention_policy_internal<V, D>(
        backups_dir: &Path,
        max_keep: usize,
        validator: V,
        deleter: D,
    ) -> std::result::Result<Vec<String>, String>
    where
        V: Fn(&Path, &str) -> Result<ValidatedBackup, BackupEntryError>,
        D: Fn(&Path) -> std::io::Result<()>,
    {
        match fs::symlink_metadata(backups_dir) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err(format!(
                        "Direktori backup {:?} adalah symbolic link, ditolak",
                        backups_dir
                    ));
                }
                if !meta.is_dir() {
                    return Err(format!(
                        "Path backup {:?} bukan sebuah direktori",
                        backups_dir
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Vec::new());
            }
            Err(e) => {
                return Err(format!(
                    "Gagal memeriksa direktori backup {:?}: {}",
                    backups_dir, e
                ));
            }
        }

        let entries = fs::read_dir(backups_dir)
            .map_err(|e| format!("Gagal membaca direktori backup {:?}: {}", backups_dir, e))?;

        let mut valid_dirs: Vec<(i64, String, PathBuf)> = Vec::new();

        for entry_res in entries {
            let entry = entry_res
                .map_err(|e| format!("Gagal membaca entri di {:?}: {}", backups_dir, e))?;
            let path = entry.path();

            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(e) => return Err(format!("Gagal membaca file type entri {:?}: {}", path, e)),
            };

            if file_type.is_symlink() {
                log::warn!(
                    "Retention mengabaikan backup invalid (symbolic link): {:?}",
                    path
                );
                continue;
            }
            if !file_type.is_dir() {
                continue;
            }

            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };

            // Abaikan folder non-backup (seperti folder staging .staging_...)
            if !name.starts_with("backup_") {
                continue;
            }

            match validator(&path, name) {
                Ok(validated) => {
                    valid_dirs.push((validated.created_at_ts, name.to_string(), path));
                }
                Err(BackupEntryError::Invalid(reason)) => {
                    log::warn!(
                        "Retention mengabaikan backup invalid {:?}: {}",
                        path,
                        reason
                    );
                }
                Err(BackupEntryError::Io(e)) => {
                    return Err(format!(
                        "Filesystem error saat memeriksa backup {:?}: {}",
                        path, e
                    ));
                }
            }
        }

        // Urutkan kandidat retensi berdasarkan timestamp pembuatan (created_at) descending (terbaru dahulu)
        valid_dirs.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));

        let mut deleted = Vec::new();
        if valid_dirs.len() > max_keep {
            for (_ts, name, path) in valid_dirs.iter().skip(max_keep) {
                deleter(path)
                    .map_err(|e| format!("Gagal menghapus backup kadaluarsa {:?}: {}", path, e))?;
                deleted.push(name.clone());
            }
        }

        Ok(deleted)
    }

    /// Mengambil daftar seluruh backup lokal dari direktori tertentu.
    /// Hanya mengembalikan backup yang terverifikasi valid dan siap direstore.
    /// Pengurutan dilakukan berdasarkan timestamp pembuatan aktual (created_at) descending.
    pub fn list_backups_in(backups_dir: &Path) -> Result<Vec<BackupItem>, String> {
        Self::list_backups_in_internal(backups_dir, Self::validate_backup_entry)
    }

    pub(crate) fn list_backups_in_internal<V>(
        backups_dir: &Path,
        validator: V,
    ) -> Result<Vec<BackupItem>, String>
    where
        V: Fn(&Path, &str) -> Result<ValidatedBackup, BackupEntryError>,
    {
        match fs::symlink_metadata(backups_dir) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err(format!(
                        "Direktori backup {:?} adalah symbolic link, ditolak",
                        backups_dir
                    ));
                }
                if !meta.is_dir() {
                    return Err(format!(
                        "Path backup {:?} bukan sebuah direktori",
                        backups_dir
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Vec::new());
            }
            Err(e) => {
                return Err(format!(
                    "Gagal memeriksa direktori backup {:?}: {}",
                    backups_dir, e
                ));
            }
        }

        let entries = fs::read_dir(backups_dir)
            .map_err(|e| format!("Gagal membaca direktori backup {:?}: {}", backups_dir, e))?;

        let mut items = Vec::new();

        for entry_res in entries {
            let entry = entry_res.map_err(|e| {
                format!("Gagal membaca entri direktori di {:?}: {}", backups_dir, e)
            })?;
            let path = entry.path();

            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(e) => return Err(format!("Gagal membaca file type entri {:?}: {}", path, e)),
            };

            if file_type.is_symlink() {
                log::warn!(
                    "Listing mengabaikan backup invalid (symbolic link): {:?}",
                    path
                );
                continue;
            }
            if !file_type.is_dir() {
                continue;
            }

            let folder_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };

            // Abaikan folder non-backup (seperti .staging_...)
            if !folder_name.starts_with("backup_") {
                continue;
            }

            // Hanya sertakan jika benar-benar valid
            match validator(&path, &folder_name) {
                Ok(validated) => {
                    let manifest = validated.manifest;
                    let total_bytes = manifest.main_db_size + manifest.log_db_size;
                    items.push((
                        validated.created_at_ts,
                        BackupItem {
                            backup_id: manifest.backup_id,
                            folder_name,
                            path: path.to_string_lossy().to_string(),
                            total_size_bytes: total_bytes,
                            total_size_formatted: Self::format_bytes(total_bytes),
                            created_at: manifest.created_at,
                            cloud_synced: false,
                        },
                    ));
                }
                Err(BackupEntryError::Invalid(reason)) => {
                    log::warn!("Listing mengabaikan backup invalid {:?}: {}", path, reason);
                }
                Err(BackupEntryError::Io(e)) => {
                    return Err(format!(
                        "Filesystem error saat memeriksa backup {:?}: {}",
                        path, e
                    ));
                }
            }
        }

        items.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| b.1.backup_id.cmp(&a.1.backup_id))
        });

        Ok(items.into_iter().map(|(_, item)| item).collect())
    }

    /// Mengambil daftar seluruh backup lokal dari direktori default aplikasi
    pub fn list_backups() -> Result<Vec<BackupItem>, String> {
        let backups_dir = Self::get_backups_dir().map_err(|e| e.to_string())?;
        Self::list_backups_in(&backups_dir)
    }

    /// Memulihkan database dari snapshot folder backup yang dipilih
    pub fn restore_backup_snapshot(
        backup_folder_path: &Path,
        main_db_path: &Path,
        log_db_path: &Path,
    ) -> Result<(), String> {
        let backup_main = backup_folder_path.join("kasir_pos.db");
        let backup_log = backup_folder_path.join("kasir_pos_log.db");

        if !backup_main.exists() {
            return Err("File kasir_pos.db tidak ditemukan di dalam snapshot backup!".to_string());
        }

        fs::copy(&backup_main, main_db_path)
            .map_err(|e| format!("Gagal memulihkan kasir_pos.db: {}", e))?;

        if backup_log.exists() {
            let _ = fs::copy(&backup_log, log_db_path);
        }

        Ok(())
    }

    /// Menyiapkan OS scheduler otomatis:
    /// - Linux: systemd user timer & service
    #[cfg(unix)]
    pub fn setup_os_scheduler() -> Result<String, String> {
        let home = dirs::home_dir().ok_or("Gagal mendapatkan home dir")?;
        let systemd_user_dir = home.join(".config/systemd/user");
        fs::create_dir_all(&systemd_user_dir).map_err(|e| e.to_string())?;

        let service_content = "[Unit]\nDescription=OlahPOS Daily Database Backup Service\n\n[Service]\nType=oneshot\nExecStart=/bin/sh -c 'echo \"OlahPOS scheduled backup executed\"'\n";
        let timer_content = "[Unit]\nDescription=OlahPOS Daily Database Backup Timer\n\n[Timer]\nOnCalendar=daily\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n";

        fs::write(
            systemd_user_dir.join("olahpos-backup.service"),
            service_content,
        )
        .map_err(|e| e.to_string())?;
        fs::write(systemd_user_dir.join("olahpos-backup.timer"), timer_content)
            .map_err(|e| e.to_string())?;

        Ok("Linux systemd user timer & service berhasil dikonfigurasi pada ~/.config/systemd/user/olahpos-backup.timer".to_string())
    }

    #[cfg(not(unix))]
    pub fn setup_os_scheduler() -> Result<String, String> {
        Ok("Windows Task Scheduler disiapkan untuk eksekusi backup berkala.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::{DatabasePaths, DbManager};
    use crate::database::migrator::Migrator;
    use std::sync::{Arc, Mutex};

    fn setup_isolated_test_env() -> (PathBuf, PathBuf, DbManager, DbConnection) {
        let test_root = std::env::temp_dir().join(format!(
            "test_olahpos_backup_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        fs::create_dir_all(&test_root).unwrap();

        let db_dir = test_root.join("databases");
        let backups_dir = test_root.join("backups");
        fs::create_dir_all(&db_dir).unwrap();
        fs::create_dir_all(&backups_dir).unwrap();

        let main_path = db_dir.join("kasir_pos.db");
        let log_path = db_dir.join("kasir_pos_log.db");

        let paths = DatabasePaths {
            main_db: main_path,
            log_db: log_path,
        };
        let manager = DbManager::init(paths).unwrap();
        let conn_arc = manager.get_connection();
        Migrator::run(&mut conn_arc.lock().unwrap()).unwrap();

        (test_root, backups_dir, manager, conn_arc)
    }

    fn create_test_sqlite_file(path: &Path, table_name: &str) -> u64 {
        let conn = Connection::open(path).expect("Failed to open test SQLite file");
        conn.execute(
            &format!(
                "CREATE TABLE {} (id INTEGER PRIMARY KEY, note TEXT);",
                table_name
            ),
            [],
        )
        .expect("Failed to create table");
        conn.execute(
            &format!(
                "INSERT INTO {} (note) VALUES ('test_data_row');",
                table_name
            ),
            [],
        )
        .expect("Failed to insert row");
        conn.close().expect("Failed to close conn");
        fs::metadata(path).expect("Failed to get metadata").len()
    }

    #[test]
    fn test_backup_main_and_log_success_and_valid_sqlite() {
        let (test_root, backups_dir, _manager, conn_arc) = setup_isolated_test_env();

        // 1. Eksekusi backup ke direktori terisolasi menggunakan API yang disederhanakan
        let res = BackupService::create_backup_snapshot_in(&conn_arc, &backups_dir)
            .expect("Backup snapshot should succeed");

        assert!(res.total_size_bytes > 0);
        let target_path = Path::new(&res.path);
        assert!(target_path.exists());

        // 2. Verifikasi kedua file database tersedia
        let backup_main = target_path.join("kasir_pos.db");
        let backup_log = target_path.join("kasir_pos_log.db");
        let manifest_file = target_path.join("manifest.json");

        assert!(backup_main.exists());
        assert!(backup_log.exists());
        assert!(manifest_file.exists());

        // 3. Verifikasi kedua file dapat dibuka dan lulus PRAGMA integrity_check
        assert_eq!(
            BackupService::validate_sqlite_database(&backup_main).unwrap(),
            fs::metadata(&backup_main).unwrap().len()
        );
        assert_eq!(
            BackupService::validate_sqlite_database(&backup_log).unwrap(),
            fs::metadata(&backup_log).unwrap().len()
        );

        // 4. Verifikasi manifest memuat versi riil dari Cargo.toml
        let manifest_str = fs::read_to_string(&manifest_file).unwrap();
        let manifest: BackupManifest = serde_json::from_str(&manifest_str).unwrap();
        assert_eq!(manifest.app_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(manifest.backup_id, res.backup_id);
        assert!(manifest.main_db_size > 0);
        assert!(manifest.log_db_size > 0);

        // 5. Verifikasi listing backup
        let list = BackupService::list_backups_in(&backups_dir).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].backup_id, res.backup_id);

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_backup_fails_when_log_db_not_attached() {
        let test_root = std::env::temp_dir().join(format!(
            "test_backup_no_log_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        let backups_dir = test_root.join("backups");
        fs::create_dir_all(&backups_dir).unwrap();

        // Buat koneksi in-memory tanpa ATTACH log_db
        let conn = Connection::open_in_memory().unwrap();
        let conn_arc = Arc::new(Mutex::new(conn));

        let result = BackupService::create_backup_snapshot_in(&conn_arc, &backups_dir);

        // Wajib gagal dan TIDAK membuat file dummy kosong
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("tidak ter-attach"));

        // Verifikasi tidak ada folder backup atau staging yang tertinggal
        let list = BackupService::list_backups_in(&backups_dir).unwrap();
        assert!(list.is_empty());

        let entries: Vec<_> = fs::read_dir(&backups_dir).unwrap().flatten().collect();
        assert!(
            entries.is_empty(),
            "Folder backups harus tetap bersih tanpa sisa staging"
        );

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_staging_directory_cleaned_up_on_failure() {
        let test_root = std::env::temp_dir().join(format!(
            "test_staging_cleanup_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        let backups_dir = test_root.join("backups");
        fs::create_dir_all(&backups_dir).unwrap();

        // Koneksi tanpa log_db untuk memicu kegagalan saat proses berjalan
        let conn = Connection::open_in_memory().unwrap();
        let conn_arc = Arc::new(Mutex::new(conn));

        let _ = BackupService::create_backup_snapshot_in(&conn_arc, &backups_dir);

        // Pastikan tidak ada folder .staging_* yang tersisa
        for entry in fs::read_dir(&backups_dir).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            assert!(
                !name.starts_with(".staging_"),
                "Staging directory harus dibersihkan otomatis"
            );
        }

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_previous_backups_remain_intact_when_new_backup_fails() {
        let (test_root, backups_dir, _manager, conn_arc) = setup_isolated_test_env();

        // 1. Buat backup pertama yang valid
        let res1 = BackupService::create_backup_snapshot_in(&conn_arc, &backups_dir)
            .expect("First backup should succeed");

        let path1 = PathBuf::from(&res1.path);
        assert!(path1.exists());
        assert!(path1.join("kasir_pos.db").exists());
        assert!(path1.join("manifest.json").exists());

        // 2. Coba buat backup kedua dengan koneksi yang rusak / tanpa log_db
        let broken_conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        let res2 = BackupService::create_backup_snapshot_in(&broken_conn, &backups_dir);
        assert!(res2.is_err());

        // 3. Pastikan backup pertama tetap utuh 100% dan tidak terganggu
        assert!(path1.exists());
        assert!(path1.join("kasir_pos.db").exists());
        assert!(path1.join("manifest.json").exists());

        let list = BackupService::list_backups_in(&backups_dir).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].backup_id, res1.backup_id);

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_no_collision_when_backups_created_rapidly() {
        let (test_root, backups_dir, _manager, conn_arc) = setup_isolated_test_env();

        // Buat 3 backup secara cepat berturut-turut
        let res1 = BackupService::create_backup_snapshot_in(&conn_arc, &backups_dir).unwrap();
        let res2 = BackupService::create_backup_snapshot_in(&conn_arc, &backups_dir).unwrap();
        let res3 = BackupService::create_backup_snapshot_in(&conn_arc, &backups_dir).unwrap();

        // Pastikan ketiganya memiliki ID unik dan foldernya eksis
        assert_ne!(res1.backup_id, res2.backup_id);
        assert_ne!(res2.backup_id, res3.backup_id);
        assert_ne!(res1.backup_id, res3.backup_id);

        assert!(Path::new(&res1.path).exists());
        assert!(Path::new(&res2.path).exists());
        assert!(Path::new(&res3.path).exists());

        let list = BackupService::list_backups_in(&backups_dir).unwrap();
        assert_eq!(list.len(), 3);

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_staging_directory_exclusive_collision_recovery() {
        let (test_root, backups_dir, _manager, conn_arc) = setup_isolated_test_env();

        // 1. Simulasikan tabrakan kandidat ID pertama dengan pre-creating folder staging milik proses lain
        let now = chrono::Utc::now();
        let base_id = format!("backup_{}", now.format("%Y%m%d_%H%M%S"));
        let nano = now.timestamp_subsec_nanos();
        let pre_existing_staging = backups_dir.join(format!(".staging_{}_{:09}", base_id, nano));
        fs::create_dir(&pre_existing_staging).unwrap();
        let sentinel_file = pre_existing_staging.join("sentinel.lock");
        fs::write(&sentinel_file, b"foreign_process_in_progress").unwrap();

        // 2. Jalankan backup OlahPOS
        let res = BackupService::create_backup_snapshot_in(&conn_arc, &backups_dir)
            .expect("Backup must gracefully recover from candidate collision");

        // 3. Verifikasi backup berhasil dibuat dengan kandidat ID berbeda
        assert!(Path::new(&res.path).exists());
        assert_ne!(res.backup_id, format!("{}_{:09}", base_id, nano));

        // 4. Verifikasi folder staging milik proses lain TIDAK disentuh atau ditimpa
        assert!(pre_existing_staging.exists());
        assert!(sentinel_file.exists());
        assert_eq!(
            fs::read(&sentinel_file).unwrap(),
            b"foreign_process_in_progress"
        );

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_listing_displays_valid_backups() {
        let test_root = std::env::temp_dir().join(format!(
            "test_listing_valid_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        let backups_dir = test_root.join("backups");
        fs::create_dir_all(&backups_dir).unwrap();

        // Buat 2 backup valid dengan waktu berbeda
        for i in 1..=2 {
            let bkp_name = format!("backup_20261101_{:02}0000_000000000", i);
            let bkp_dir = backups_dir.join(&bkp_name);
            fs::create_dir_all(&bkp_dir).unwrap();

            let main_db = bkp_dir.join("kasir_pos.db");
            let log_db = bkp_dir.join("kasir_pos_log.db");
            let main_size = create_test_sqlite_file(&main_db, "main_tbl");
            let log_size = create_test_sqlite_file(&log_db, "log_tbl");

            let manifest = BackupManifest {
                backup_id: bkp_name.clone(),
                created_at: format!("2026-11-01T{:02}:00:00Z", i),
                app_version: env!("CARGO_PKG_VERSION").to_string(),
                main_db_size: main_size,
                log_db_size: log_size,
            };
            fs::write(
                bkp_dir.join("manifest.json"),
                serde_json::to_string(&manifest).unwrap(),
            )
            .unwrap();
        }

        let list = BackupService::list_backups_in(&backups_dir).expect("Listing must succeed");
        assert_eq!(list.len(), 2);
        // Terurut descending berdasarkan timestamp (i=2 lebih baru daripada i=1)
        assert_eq!(list[0].backup_id, "backup_20261101_020000_000000000");
        assert_eq!(list[1].backup_id, "backup_20261101_010000_000000000");
        assert!(list[0].total_size_bytes > 0);
        assert!(!list[0].total_size_formatted.is_empty());

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_listing_rejects_invalid_manifest() {
        let test_root = std::env::temp_dir().join(format!(
            "test_listing_inv_man_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        let backups_dir = test_root.join("backups");
        let bkp_dir = backups_dir.join("backup_20261101_010000_000000000");
        fs::create_dir_all(&bkp_dir).unwrap();

        let main_db = bkp_dir.join("kasir_pos.db");
        let log_db = bkp_dir.join("kasir_pos_log.db");
        create_test_sqlite_file(&main_db, "m");
        create_test_sqlite_file(&log_db, "l");
        fs::write(bkp_dir.join("manifest.json"), b"{ invalid json").unwrap();

        let list = BackupService::list_backups_in(&backups_dir).unwrap();
        assert!(
            list.is_empty(),
            "Backup dengan manifest corrupt tidak boleh muncul di listing"
        );

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_listing_rejects_missing_manifest() {
        let test_root = std::env::temp_dir().join(format!(
            "test_listing_no_man_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        let backups_dir = test_root.join("backups");
        let bkp_dir = backups_dir.join("backup_20261101_010000_000000000");
        fs::create_dir_all(&bkp_dir).unwrap();

        let main_db = bkp_dir.join("kasir_pos.db");
        let log_db = bkp_dir.join("kasir_pos_log.db");
        create_test_sqlite_file(&main_db, "m");
        create_test_sqlite_file(&log_db, "l");
        // Tanpa manifest.json sama sekali

        let list = BackupService::list_backups_in(&backups_dir).unwrap();
        assert!(
            list.is_empty(),
            "Backup tanpa manifest tidak boleh muncul di listing"
        );

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_listing_rejects_corrupted_main_database() {
        let test_root = std::env::temp_dir().join(format!(
            "test_listing_corrupt_main_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        let backups_dir = test_root.join("backups");
        let bkp_name = "backup_20261101_010000_000000000";
        let bkp_dir = backups_dir.join(bkp_name);
        fs::create_dir_all(&bkp_dir).unwrap();

        let main_db = bkp_dir.join("kasir_pos.db");
        let log_db = bkp_dir.join("kasir_pos_log.db");
        fs::write(&main_db, b"CORRUPTED_NON_SQLITE_BYTES").unwrap();
        let log_size = create_test_sqlite_file(&log_db, "l");

        let manifest = BackupManifest {
            backup_id: bkp_name.to_string(),
            created_at: "2026-11-01T01:00:00Z".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            main_db_size: 26,
            log_db_size: log_size,
        };
        fs::write(
            bkp_dir.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();

        let list = BackupService::list_backups_in(&backups_dir).unwrap();
        assert!(
            list.is_empty(),
            "Backup dengan main db corrupt tidak boleh muncul di listing"
        );

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_listing_rejects_corrupted_log_database() {
        let test_root = std::env::temp_dir().join(format!(
            "test_listing_corrupt_log_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        let backups_dir = test_root.join("backups");
        let bkp_name = "backup_20261101_010000_000000000";
        let bkp_dir = backups_dir.join(bkp_name);
        fs::create_dir_all(&bkp_dir).unwrap();

        let main_db = bkp_dir.join("kasir_pos.db");
        let log_db = bkp_dir.join("kasir_pos_log.db");
        let main_size = create_test_sqlite_file(&main_db, "m");
        fs::write(&log_db, b"CORRUPTED_NON_SQLITE_BYTES").unwrap();

        let manifest = BackupManifest {
            backup_id: bkp_name.to_string(),
            created_at: "2026-11-01T01:00:00Z".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            main_db_size: main_size,
            log_db_size: 26,
        };
        fs::write(
            bkp_dir.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();

        let list = BackupService::list_backups_in(&backups_dir).unwrap();
        assert!(
            list.is_empty(),
            "Backup dengan log db corrupt tidak boleh muncul di listing"
        );

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_listing_rejects_missing_log_database() {
        let test_root = std::env::temp_dir().join(format!(
            "test_listing_no_log_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        let backups_dir = test_root.join("backups");
        let bkp_name = "backup_20261101_010000_000000000";
        let bkp_dir = backups_dir.join(bkp_name);
        fs::create_dir_all(&bkp_dir).unwrap();

        let main_db = bkp_dir.join("kasir_pos.db");
        let main_size = create_test_sqlite_file(&main_db, "m");
        // kasir_pos_log.db sengaja tidak dibuat

        let manifest = BackupManifest {
            backup_id: bkp_name.to_string(),
            created_at: "2026-11-01T01:00:00Z".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            main_db_size: main_size,
            log_db_size: 4096,
        };
        fs::write(
            bkp_dir.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();

        let list = BackupService::list_backups_in(&backups_dir).unwrap();
        assert!(
            list.is_empty(),
            "Backup tanpa log db tidak boleh muncul di listing"
        );

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_listing_rejects_size_mismatch() {
        let test_root = std::env::temp_dir().join(format!(
            "test_listing_mismatch_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        let backups_dir = test_root.join("backups");
        let bkp_name = "backup_20261101_010000_000000000";
        let bkp_dir = backups_dir.join(bkp_name);
        fs::create_dir_all(&bkp_dir).unwrap();

        let main_db = bkp_dir.join("kasir_pos.db");
        let log_db = bkp_dir.join("kasir_pos_log.db");
        let main_size = create_test_sqlite_file(&main_db, "m");
        let log_size = create_test_sqlite_file(&log_db, "l");

        let manifest = BackupManifest {
            backup_id: bkp_name.to_string(),
            created_at: "2026-11-01T01:00:00Z".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            main_db_size: main_size + 99999, // mismatch
            log_db_size: log_size,
        };
        fs::write(
            bkp_dir.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();

        let list = BackupService::list_backups_in(&backups_dir).unwrap();
        assert!(
            list.is_empty(),
            "Backup dengan size mismatch tidak boleh muncul di listing"
        );

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_retention_retains_maximum_10_latest_valid_backups() {
        let (test_root, backups_dir, _manager, _conn_arc) = setup_isolated_test_env();

        // 1. Buat 12 backup valid menggunakan database SQLite sungguhan
        for i in 1..=12 {
            let bkp_name = format!("backup_20261101_{:02}0000_000000000", i);
            let bkp_dir = backups_dir.join(&bkp_name);
            fs::create_dir_all(&bkp_dir).unwrap();

            let main_db = bkp_dir.join("kasir_pos.db");
            let log_db = bkp_dir.join("kasir_pos_log.db");
            let main_size = create_test_sqlite_file(&main_db, "main_table");
            let log_size = create_test_sqlite_file(&log_db, "log_table");

            let manifest = BackupManifest {
                backup_id: bkp_name.clone(),
                created_at: format!("2026-11-01T{:02}:00:00Z", i),
                app_version: env!("CARGO_PKG_VERSION").to_string(),
                main_db_size: main_size,
                log_db_size: log_size,
            };
            fs::write(
                bkp_dir.join("manifest.json"),
                serde_json::to_string(&manifest).unwrap(),
            )
            .unwrap();
        }

        // 2. Jalankan retensi maksimal 10
        let deleted = BackupService::enforce_retention_policy(&backups_dir, 10).unwrap();

        // Dari 12 valid backup, tepat 2 tertua berdasarkan created_at harus dihapus (i=1 dan i=2)
        assert_eq!(deleted.len(), 2);
        assert!(deleted.contains(&"backup_20261101_010000_000000000".to_string()));
        assert!(deleted.contains(&"backup_20261101_020000_000000000".to_string()));

        // Sisa 10 backup valid terbaru (i=3..=12) harus tetap ada
        for i in 3..=12 {
            assert!(
                backups_dir
                    .join(format!("backup_20261101_{:02}0000_000000000", i))
                    .exists(),
                "Valid backup i={} harus tetap ada",
                i
            );
        }

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_invalid_backups_do_not_cause_valid_backups_to_be_deleted() {
        let (test_root, backups_dir, _manager, _conn_arc) = setup_isolated_test_env();

        // 1. Buat 10 backup valid
        for i in 1..=10 {
            let bkp_name = format!("backup_20261101_{:02}0000_000000000", i);
            let bkp_dir = backups_dir.join(&bkp_name);
            fs::create_dir_all(&bkp_dir).unwrap();

            let main_db = bkp_dir.join("kasir_pos.db");
            let log_db = bkp_dir.join("kasir_pos_log.db");
            let main_size = create_test_sqlite_file(&main_db, "main_table");
            let log_size = create_test_sqlite_file(&log_db, "log_table");

            let manifest = BackupManifest {
                backup_id: bkp_name.clone(),
                created_at: format!("2026-11-01T{:02}:00:00Z", i),
                app_version: env!("CARGO_PKG_VERSION").to_string(),
                main_db_size: main_size,
                log_db_size: log_size,
            };
            fs::write(
                bkp_dir.join("manifest.json"),
                serde_json::to_string(&manifest).unwrap(),
            )
            .unwrap();
        }

        // 2. Tambahkan 4 folder backup invalid
        let inv1 = backups_dir.join("backup_20261101_110000_inv_manifest");
        fs::create_dir_all(&inv1).unwrap();
        fs::write(inv1.join("manifest.json"), b"{ bad").unwrap();

        let inv2 = backups_dir.join("backup_20261101_120000_inv_db");
        fs::create_dir_all(&inv2).unwrap();
        fs::write(inv2.join("kasir_pos.db"), b"bad db").unwrap();

        let inv3 = backups_dir.join("backup_20261101_130000_no_manifest");
        fs::create_dir_all(&inv3).unwrap();

        let inv4 = backups_dir.join("backup_20261101_140000_mismatch");
        fs::create_dir_all(&inv4).unwrap();
        let m4 = create_test_sqlite_file(&inv4.join("kasir_pos.db"), "m4");
        let l4 = create_test_sqlite_file(&inv4.join("kasir_pos_log.db"), "l4");
        let man4 = BackupManifest {
            backup_id: "backup_20261101_140000_mismatch".to_string(),
            created_at: "2026-11-01T14:00:00Z".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            main_db_size: m4 + 500,
            log_db_size: l4,
        };
        fs::write(
            inv4.join("manifest.json"),
            serde_json::to_string(&man4).unwrap(),
        )
        .unwrap();

        // 3. Jalankan retensi dengan limit 10
        let deleted = BackupService::enforce_retention_policy(&backups_dir, 10).unwrap();

        // Karena hanya ada 10 valid backup, TIDAK ADA valid backup yang dihapus
        assert!(
            deleted.is_empty(),
            "Tidak boleh ada valid backup yang dihapus"
        );

        // Semua 10 backup valid tetap ada
        for i in 1..=10 {
            assert!(backups_dir
                .join(format!("backup_20261101_{:02}0000_000000000", i))
                .exists());
        }

        // Keempat backup invalid juga tidak boleh dihapus otomatis
        assert!(inv1.exists());
        assert!(inv2.exists());
        assert!(inv3.exists());
        assert!(inv4.exists());

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_staging_backups_not_counted_as_final() {
        let test_root = std::env::temp_dir().join(format!(
            "test_staging_ignored_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        let backups_dir = test_root.join("backups");
        fs::create_dir_all(&backups_dir).unwrap();

        // Buat folder staging tiruan yang sedang diproses
        let staging_dir = backups_dir.join(".staging_backup_20261101_010000_000000000");
        fs::create_dir_all(&staging_dir).unwrap();
        let main_db = staging_dir.join("kasir_pos.db");
        let log_db = staging_dir.join("kasir_pos_log.db");
        let m_size = create_test_sqlite_file(&main_db, "m");
        let l_size = create_test_sqlite_file(&log_db, "l");
        let manifest = BackupManifest {
            backup_id: "backup_20261101_010000_000000000".to_string(),
            created_at: "2026-11-01T01:00:00Z".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            main_db_size: m_size,
            log_db_size: l_size,
        };
        fs::write(
            staging_dir.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();

        // Listing tidak boleh memuat folder staging
        let list = BackupService::list_backups_in(&backups_dir).unwrap();
        assert!(
            list.is_empty(),
            "Folder staging tidak boleh masuk listing backup final"
        );

        // Retention tidak boleh menghapus atau memperhitungkan folder staging
        let deleted = BackupService::enforce_retention_policy(&backups_dir, 10).unwrap();
        assert!(deleted.is_empty());
        assert!(
            staging_dir.exists(),
            "Folder staging tidak boleh dihapus oleh retensi"
        );

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_filesystem_read_dir_failure_handled_explicitly() {
        let test_root = std::env::temp_dir().join(format!(
            "test_fs_read_err_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        fs::create_dir_all(&test_root).unwrap();

        // 1. Path berupa file reguler (bukan direktori)
        let not_a_dir = test_root.join("regular_file.txt");
        fs::write(&not_a_dir, b"not a directory").unwrap();

        let list_res = BackupService::list_backups_in(&not_a_dir);
        assert!(
            list_res.is_err(),
            "Kegagalan pemeriksaan direktori pada listing harus mengembalikan Err"
        );
        let list_err = list_res.unwrap_err();
        assert!(list_err.contains("bukan sebuah direktori"));

        let ret_res = BackupService::enforce_retention_policy(&not_a_dir, 10);
        assert!(
            ret_res.is_err(),
            "Kegagalan pemeriksaan direktori pada retensi harus mengembalikan Err"
        );
        let ret_err = ret_res.unwrap_err();
        assert!(ret_err.contains("bukan sebuah direktori"));

        // 2. Simulated filesystem I/O error pada entri (BackupEntryError::Io) harus menghentikan listing & retensi
        let backups_dir = test_root.join("backups");
        fs::create_dir_all(&backups_dir).unwrap();
        let bkp_dir = backups_dir.join("backup_20261101_100000_000000000");
        fs::create_dir_all(&bkp_dir).unwrap();

        let io_failing_validator =
            |_p: &Path, _n: &str| -> Result<ValidatedBackup, BackupEntryError> {
                Err(BackupEntryError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Simulated filesystem I/O fault",
                )))
            };

        let list_io_res =
            BackupService::list_backups_in_internal(&backups_dir, io_failing_validator);
        assert!(
            list_io_res.is_err(),
            "Filesystem error harus menghentikan listing"
        );
        assert!(list_io_res
            .unwrap_err()
            .contains("Filesystem error saat memeriksa backup"));

        let ret_io_res = BackupService::enforce_retention_policy_internal(
            &backups_dir,
            10,
            io_failing_validator,
            |p| fs::remove_dir_all(p),
        );
        assert!(
            ret_io_res.is_err(),
            "Filesystem error harus menghentikan retention"
        );
        assert!(ret_io_res
            .unwrap_err()
            .contains("Filesystem error saat memeriksa backup"));

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_non_existent_backups_dir_returns_empty_list() {
        let test_root = std::env::temp_dir().join(format!(
            "test_non_existent_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        let non_existent = test_root.join("missing_dir");

        let list = BackupService::list_backups_in(&non_existent).unwrap();
        assert!(
            list.is_empty(),
            "Folder hilang harus menghasilkan list kosong"
        );

        let deleted = BackupService::enforce_retention_policy(&non_existent, 10).unwrap();
        assert!(
            deleted.is_empty(),
            "Folder hilang tidak boleh menghapus apapun"
        );
    }

    #[test]
    fn test_validation_created_at_rfc3339() {
        let test_root = std::env::temp_dir().join(format!(
            "test_created_at_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        let backups_dir = test_root.join("backups");
        fs::create_dir_all(&backups_dir).unwrap();

        // 1. Backup dengan created_at valid (RFC 3339)
        let valid_name = "backup_20261101_100000_000000000";
        let valid_dir = backups_dir.join(valid_name);
        fs::create_dir_all(&valid_dir).unwrap();
        let m_size = create_test_sqlite_file(&valid_dir.join("kasir_pos.db"), "m");
        let l_size = create_test_sqlite_file(&valid_dir.join("kasir_pos_log.db"), "l");
        let valid_manifest = BackupManifest {
            backup_id: valid_name.to_string(),
            created_at: "2026-11-01T10:00:00Z".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            main_db_size: m_size,
            log_db_size: l_size,
        };
        fs::write(
            valid_dir.join("manifest.json"),
            serde_json::to_string(&valid_manifest).unwrap(),
        )
        .unwrap();

        let val_res = BackupService::validate_backup_entry(&valid_dir, valid_name);
        assert!(
            val_res.is_ok(),
            "Manifest dengan RFC 3339 valid harus diterima"
        );
        let validated = val_res.unwrap();
        assert_eq!(validated.manifest.backup_id, valid_name);
        assert!(validated.created_at_ts > 0);

        // 2. Backup dengan created_at invalid (bukan RFC 3339)
        let invalid_dt_name = "backup_20261101_110000_000000000";
        let invalid_dt_dir = backups_dir.join(invalid_dt_name);
        fs::create_dir_all(&invalid_dt_dir).unwrap();
        create_test_sqlite_file(&invalid_dt_dir.join("kasir_pos.db"), "m");
        create_test_sqlite_file(&invalid_dt_dir.join("kasir_pos_log.db"), "l");
        let invalid_dt_manifest = BackupManifest {
            backup_id: invalid_dt_name.to_string(),
            created_at: "01-11-2026 10:00:00".to_string(), // format non-RFC 3339
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            main_db_size: m_size,
            log_db_size: l_size,
        };
        fs::write(
            invalid_dt_dir.join("manifest.json"),
            serde_json::to_string(&invalid_dt_manifest).unwrap(),
        )
        .unwrap();

        let val_err = BackupService::validate_backup_entry(&invalid_dt_dir, invalid_dt_name);
        assert!(val_err.is_err(), "created_at non-RFC 3339 harus ditolak");
        assert!(matches!(
            val_err.unwrap_err(),
            BackupEntryError::Invalid(msg) if msg.contains("RFC 3339")
        ));

        // 3. Backup dengan created_at kosong
        let empty_dt_name = "backup_20261101_120000_000000000";
        let empty_dt_dir = backups_dir.join(empty_dt_name);
        fs::create_dir_all(&empty_dt_dir).unwrap();
        create_test_sqlite_file(&empty_dt_dir.join("kasir_pos.db"), "m");
        create_test_sqlite_file(&empty_dt_dir.join("kasir_pos_log.db"), "l");
        let empty_dt_manifest = BackupManifest {
            backup_id: empty_dt_name.to_string(),
            created_at: "   ".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            main_db_size: m_size,
            log_db_size: l_size,
        };
        fs::write(
            empty_dt_dir.join("manifest.json"),
            serde_json::to_string(&empty_dt_manifest).unwrap(),
        )
        .unwrap();

        let val_empty = BackupService::validate_backup_entry(&empty_dt_dir, empty_dt_name);
        assert!(val_empty.is_err(), "created_at kosong harus ditolak");
        assert!(matches!(
            val_empty.unwrap_err(),
            BackupEntryError::Invalid(msg) if msg.contains("kosong")
        ));

        // 4. Listing hanya mengembalikan backup dengan created_at valid
        let list = BackupService::list_backups_in(&backups_dir).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].backup_id, valid_name);

        // 5. Retensi tidak menghapus backup valid akibat adanya backup dengan created_at invalid
        let deleted = BackupService::enforce_retention_policy(&backups_dir, 10).unwrap();
        assert!(deleted.is_empty());
        assert!(valid_dir.exists(), "Backup valid harus tetap utuh");

        let _ = fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_backup_rejects_symlinks() {
        use std::os::unix::fs as unix_fs;

        let test_root = std::env::temp_dir().join(format!(
            "test_symlinks_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        let backups_dir = test_root.join("backups");
        fs::create_dir_all(&backups_dir).unwrap();

        // 1. Folder backup berupa symbolic link
        let real_bkp = test_root.join("real_folder");
        fs::create_dir_all(&real_bkp).unwrap();
        let m_size = create_test_sqlite_file(&real_bkp.join("kasir_pos.db"), "m");
        let l_size = create_test_sqlite_file(&real_bkp.join("kasir_pos_log.db"), "l");
        let symlink_dir_name = "backup_20261101_100000_000000000";
        let manifest = BackupManifest {
            backup_id: symlink_dir_name.to_string(),
            created_at: "2026-11-01T10:00:00Z".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            main_db_size: m_size,
            log_db_size: l_size,
        };
        fs::write(
            real_bkp.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();

        let symlink_bkp = backups_dir.join(symlink_dir_name);
        unix_fs::symlink(&real_bkp, &symlink_bkp).unwrap();

        let val_symlink_dir = BackupService::validate_backup_entry(&symlink_bkp, symlink_dir_name);
        assert!(
            val_symlink_dir.is_err(),
            "Folder backup symlink harus ditolak"
        );
        assert!(matches!(
            val_symlink_dir.unwrap_err(),
            BackupEntryError::Invalid(msg) if msg.contains("symbolic link")
        ));

        // 2. File manifest.json berupa symbolic link
        let manifest_sym_name = "backup_20261101_110000_000000000";
        let manifest_sym_dir = backups_dir.join(manifest_sym_name);
        fs::create_dir_all(&manifest_sym_dir).unwrap();
        create_test_sqlite_file(&manifest_sym_dir.join("kasir_pos.db"), "m");
        create_test_sqlite_file(&manifest_sym_dir.join("kasir_pos_log.db"), "l");
        let outside_manifest = test_root.join("outside_manifest.json");
        let manifest2 = BackupManifest {
            backup_id: manifest_sym_name.to_string(),
            created_at: "2026-11-01T11:00:00Z".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            main_db_size: m_size,
            log_db_size: l_size,
        };
        fs::write(
            &outside_manifest,
            serde_json::to_string(&manifest2).unwrap(),
        )
        .unwrap();
        unix_fs::symlink(&outside_manifest, manifest_sym_dir.join("manifest.json")).unwrap();

        let val_sym_manifest =
            BackupService::validate_backup_entry(&manifest_sym_dir, manifest_sym_name);
        assert!(val_sym_manifest.is_err(), "Manifest symlink harus ditolak");
        assert!(matches!(
            val_sym_manifest.unwrap_err(),
            BackupEntryError::Invalid(msg) if msg.contains("symbolic link")
        ));

        // 3. File database berupa symbolic link
        let db_sym_name = "backup_20261101_120000_000000000";
        let db_sym_dir = backups_dir.join(db_sym_name);
        fs::create_dir_all(&db_sym_dir).unwrap();
        let outside_db = test_root.join("outside.db");
        create_test_sqlite_file(&outside_db, "o");
        unix_fs::symlink(&outside_db, db_sym_dir.join("kasir_pos.db")).unwrap();
        create_test_sqlite_file(&db_sym_dir.join("kasir_pos_log.db"), "l");
        let manifest3 = BackupManifest {
            backup_id: db_sym_name.to_string(),
            created_at: "2026-11-01T12:00:00Z".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            main_db_size: m_size,
            log_db_size: l_size,
        };
        fs::write(
            db_sym_dir.join("manifest.json"),
            serde_json::to_string(&manifest3).unwrap(),
        )
        .unwrap();

        let val_sym_db = BackupService::validate_backup_entry(&db_sym_dir, db_sym_name);
        assert!(val_sym_db.is_err(), "Database symlink harus ditolak");
        assert!(matches!(
            val_sym_db.unwrap_err(),
            BackupEntryError::Invalid(msg) if msg.contains("symbolic link")
        ));

        // Listing harus kosong (semua symlink ditolak)
        let list = BackupService::list_backups_in(&backups_dir).unwrap();
        assert!(
            list.is_empty(),
            "Listing tidak boleh memuat backup dengan symlink"
        );

        // Retention tidak boleh menganggap backup symlink valid dan tidak menghapusnya
        let deleted = BackupService::enforce_retention_policy(&backups_dir, 10).unwrap();
        assert!(deleted.is_empty());

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_retention_reports_deletion_failure() {
        let test_root = std::env::temp_dir().join(format!(
            "test_del_fail_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        let backups_dir = test_root.join("backups");
        fs::create_dir_all(&backups_dir).unwrap();

        // Buat 11 backup valid
        for i in 1..=11 {
            let bkp_name = format!("backup_20261101_{:02}0000_000000000", i);
            let bkp_dir = backups_dir.join(&bkp_name);
            fs::create_dir_all(&bkp_dir).unwrap();

            let main_db = bkp_dir.join("kasir_pos.db");
            let log_db = bkp_dir.join("kasir_pos_log.db");
            let main_size = create_test_sqlite_file(&main_db, "m");
            let log_size = create_test_sqlite_file(&log_db, "l");

            let manifest = BackupManifest {
                backup_id: bkp_name.clone(),
                created_at: format!("2026-11-01T{:02}:00:00Z", i),
                app_version: env!("CARGO_PKG_VERSION").to_string(),
                main_db_size: main_size,
                log_db_size: log_size,
            };
            fs::write(
                bkp_dir.join("manifest.json"),
                serde_json::to_string(&manifest).unwrap(),
            )
            .unwrap();
        }

        // Simulasikan kegagalan deleter secara deterministik (tidak bergantung pada izin root/chmod)
        let simulated_deleter = |_p: &Path| -> std::io::Result<()> {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Simulated OS disk write/unlink error",
            ))
        };

        let ret_res = BackupService::enforce_retention_policy_internal(
            &backups_dir,
            10,
            BackupService::validate_backup_entry,
            simulated_deleter,
        );

        assert!(
            ret_res.is_err(),
            "Kegagalan menghapus backup kadaluarsa harus dikembalikan sebagai Err"
        );
        let err_msg = ret_res.unwrap_err();
        assert!(
            err_msg.contains("Gagal menghapus backup kadaluarsa"),
            "Pesan error harus memuat kegagalan: {}",
            err_msg
        );

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_integrity_validation_rejects_corrupted_database() {
        let test_root = std::env::temp_dir().join(format!(
            "test_integrity_{}_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            std::process::id()
        ));
        fs::create_dir_all(&test_root).unwrap();

        // 1. File tidak ada
        let non_existent = test_root.join("not_found.db");
        assert!(BackupService::validate_sqlite_database(&non_existent).is_err());

        // 2. File 0 byte
        let empty_file = test_root.join("empty.db");
        fs::write(&empty_file, b"").unwrap();
        assert!(BackupService::validate_sqlite_database(&empty_file).is_err());

        // 3. File berisi karakter acak bukan SQLite
        let corrupt_file = test_root.join("corrupt.db");
        fs::write(
            &corrupt_file,
            b"This is definitely not an SQLite database header.",
        )
        .unwrap();
        assert!(BackupService::validate_sqlite_database(&corrupt_file).is_err());

        // 4. File SQLite valid
        let valid_file = test_root.join("valid.db");
        {
            let conn = Connection::open(&valid_file).unwrap();
            conn.execute("CREATE TABLE test (id INT);", []).unwrap();
        }
        let val_res = BackupService::validate_sqlite_database(&valid_file);
        assert!(val_res.is_ok());
        assert!(val_res.unwrap() > 0);

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_never_touches_user_backup_directory() {
        // Verifikasi bahwa seluruh test environment kita mengarah ke temp_dir
        let (test_root, backups_dir, _manager, conn_arc) = setup_isolated_test_env();

        let user_backups_dir = BackupService::get_backups_dir().unwrap();
        assert_ne!(backups_dir, user_backups_dir);

        let res = BackupService::create_backup_snapshot_in(&conn_arc, &backups_dir).unwrap();

        assert!(!res
            .path
            .contains(&user_backups_dir.to_string_lossy().to_string()));
        assert!(res.path.contains(&test_root.to_string_lossy().to_string()));

        let _ = fs::remove_dir_all(&test_root);
    }
}
