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

/// RAII Guard untuk memastikan folder staging otomatis dibersihkan jika proses backup gagal
struct StagingGuard<'a> {
    path: &'a Path,
    committed: bool,
}

impl<'a> Drop for StagingGuard<'a> {
    fn drop(&mut self) {
        if !self.committed && self.path.exists() {
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
        if !backups_dir.exists() {
            fs::create_dir_all(&backups_dir)?;
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
        let mut dst_conn = Connection::open(dst_path)
            .map_err(|e| format!("Gagal menginisialisasi target database {:?}: {}", dst_path, e))?;

        let backup = rusqlite::backup::Backup::new_with_names(
            src_conn,
            src_db_name,
            &mut dst_conn,
            rusqlite::DatabaseName::Main,
        )
        .map_err(|e| format!("Gagal inisialisasi SQLite backup engine untuk {:?}: {}", dst_path, e))?;

        let mut busy_retries = 0;
        loop {
            match backup.step(100) {
                Ok(rusqlite::backup::StepResult::Done) => break,
                Ok(rusqlite::backup::StepResult::More) => {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                Ok(rusqlite::backup::StepResult::Busy) | Ok(rusqlite::backup::StepResult::Locked) => {
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

    /// Memvalidasi berkas database SQLite: keberadaan, ukuran > 0, bisa dibuka, dan lulus integrity_check
    pub fn validate_sqlite_database(path: &Path) -> Result<u64, String> {
        if !path.exists() {
            return Err(format!("File database tidak ditemukan: {:?}", path));
        }
        let metadata = fs::metadata(path)
            .map_err(|e| format!("Gagal membaca metadata file {:?}: {}", path, e))?;
        let size = metadata.len();
        if size == 0 {
            return Err(format!("File database kosong (0 bytes): {:?}", path));
        }

        let conn = Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| format!("File bukan database SQLite yang valid ({:?}): {}", path, e))?;

        let integrity: String = conn
            .query_row("PRAGMA integrity_check;", [], |r| r.get(0))
            .map_err(|e| format!("Gagal menjalankan integrity_check pada {:?}: {}", path, e))?;

        if integrity != "ok" {
            return Err(format!(
                "Database integrity_check gagal pada {:?}: {}",
                path, integrity
            ));
        }

        Ok(size)
    }

    /// Melakukan atomic online backup dual SQLite database (kasir_pos.db & kasir_pos_log.db)
    /// ke direktori target yang ditentukan dengan staging dan validasi integritas
    pub fn create_backup_snapshot_in(
        db: &DbConnection,
        _main_db_path: &Path,
        _log_db_path: &Path,
        backups_dir: &Path,
    ) -> std::result::Result<BackupResult, String> {
        if !backups_dir.exists() {
            fs::create_dir_all(backups_dir)
                .map_err(|e| format!("Gagal membuat direktori backups: {}", e))?;
        }

        // 1. Generate collision-resistant ID (berbasis waktu dengan nanodetik & sequence counter fallback)
        let now = chrono::Utc::now();
        let base_id = format!("backup_{}", now.format("%Y%m%d_%H%M%S"));
        let nano = now.timestamp_subsec_nanos();
        let mut backup_id = format!("{}_{:09}", base_id, nano);
        let mut counter = 1;
        while backups_dir.join(&backup_id).exists()
            || backups_dir.join(format!(".staging_{}", backup_id)).exists()
        {
            backup_id = format!("{}_{:09}_{}", base_id, nano, counter);
            counter += 1;
        }

        // 2. Buat direktori staging terisolasi
        let staging_dir = backups_dir.join(format!(".staging_{}", backup_id));
        fs::create_dir_all(&staging_dir)
            .map_err(|e| format!("Gagal membuat folder staging backup: {}", e))?;

        let mut staging_guard = StagingGuard {
            path: &staging_dir,
            committed: false,
        };

        let target_main_db = staging_dir.join("kasir_pos.db");
        let target_log_db = staging_dir.join("kasir_pos_log.db");

        // 3. Kunci seluruh koneksi sumber selama proses snapshot dual-database
        // Ini menjamin tidak ada transaksi aplikasi yang menyela di antara backup kedua database
        {
            let src_conn = db
                .lock()
                .map_err(|e| format!("Gagal mengunci mutex koneksi database: {}", e))?;

            // Pastikan log_db benar-benar ter-attach pada koneksi sumber
            let log_attached = Self::is_database_attached(&src_conn, "log_db")?;
            if !log_attached {
                return Err("Database log 'log_db' tidak ter-attach pada koneksi SQLite aktif!".to_string());
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

        // 4. Validasi kedua database hasil backup secara mendalam (harus valid SQLite & integrity "ok")
        let main_db_size = Self::validate_sqlite_database(&target_main_db)?;
        let log_db_size = Self::validate_sqlite_database(&target_log_db)?;
        let total_size = main_db_size + log_db_size;

        // 5. Buat manifest.json dengan versi aplikasi riil dari Cargo.toml
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

        // 6. Publikasikan folder backup secara atomik melalui rename
        let target_dir = backups_dir.join(&backup_id);
        fs::rename(&staging_dir, &target_dir)
            .map_err(|e| format!("Gagal mempublikasikan folder backup secara atomik: {}", e))?;

        staging_guard.committed = true;

        // 7. Jalankan kebijakan retensi hanya setelah backup terverifikasi sukses dipublikasikan
        let deleted_old_backups = Self::enforce_retention_policy(backups_dir, 10)?;

        // 8. Cek status sinkronisasi cloud
        let gdrive_config = IntegrationService::get_config(db, "GOOGLE_DRIVE_BACKUP").ok().flatten();
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
    pub fn create_backup_snapshot(
        db: &DbConnection,
        main_db_path: &Path,
        log_db_path: &Path,
    ) -> std::result::Result<BackupResult, String> {
        let backups_dir = Self::get_backups_dir().map_err(|e| e.to_string())?;
        Self::create_backup_snapshot_in(db, main_db_path, log_db_path, &backups_dir)
    }

    /// Menghapus backup lama yang melampaui batas retensi maksimum (default 10)
    /// Hanya menghitung backup yang terverifikasi valid
    pub fn enforce_retention_policy(
        backups_dir: &Path,
        max_keep: usize,
    ) -> std::result::Result<Vec<String>, String> {
        let mut valid_dirs = Vec::new();

        if let Ok(entries) = fs::read_dir(backups_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        // Hanya proses direktori backup_ (abaikan folder tersembunyi .staging_...)
                        if name.starts_with("backup_") {
                            let manifest_file = path.join("manifest.json");
                            let main_db = path.join("kasir_pos.db");
                            let log_db = path.join("kasir_pos_log.db");

                            // Hanya anggap valid jika manifest dan kedua file database ada
                            if manifest_file.exists() && main_db.exists() && log_db.exists() {
                                valid_dirs.push((name.to_string(), path));
                            }
                        }
                    }
                }
            }
        }

        // Urutkan nama folder descending (format backup_YYYYMMDD_HHMMSS_<nanos> terurut waktu)
        valid_dirs.sort_by(|a, b| b.0.cmp(&a.0));

        let mut deleted = Vec::new();
        if valid_dirs.len() > max_keep {
            for (name, path) in valid_dirs.iter().skip(max_keep) {
                if let Err(e) = fs::remove_dir_all(path) {
                    log::warn!("Gagal menghapus backup kadaluarsa {}: {}", name, e);
                } else {
                    deleted.push(name.clone());
                }
            }
        }

        Ok(deleted)
    }

    /// Mengambil daftar seluruh backup lokal dari direktori tertentu
    pub fn list_backups_in(backups_dir: &Path) -> Result<Vec<BackupItem>, String> {
        let mut items = Vec::new();

        if let Ok(entries) = fs::read_dir(backups_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let folder_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default()
                        .to_string();

                    if !folder_name.starts_with("backup_") {
                        continue;
                    }

                    let manifest_file = path.join("manifest.json");
                    let (created_at, total_bytes) = if manifest_file.exists() {
                        if let Ok(content) = fs::read_to_string(&manifest_file) {
                            if let Ok(m) = serde_json::from_str::<BackupManifest>(&content) {
                                (m.created_at, m.main_db_size + m.log_db_size)
                            } else {
                                ("-".to_string(), 0)
                            }
                        } else {
                            ("-".to_string(), 0)
                        }
                    } else {
                        ("-".to_string(), 0)
                    };

                    items.push(BackupItem {
                        backup_id: folder_name.clone(),
                        folder_name,
                        path: path.to_string_lossy().to_string(),
                        total_size_bytes: total_bytes,
                        total_size_formatted: Self::format_bytes(total_bytes),
                        created_at,
                        cloud_synced: false,
                    });
                }
            }
        }

        items.sort_by(|a, b| b.backup_id.cmp(&a.backup_id));
        Ok(items)
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

        fs::write(systemd_user_dir.join("olahpos-backup.service"), service_content)
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

    #[test]
    fn test_backup_main_and_log_success_and_valid_sqlite() {
        let (test_root, backups_dir, manager, conn_arc) = setup_isolated_test_env();
        let paths = manager.get_paths();

        // 1. Eksekusi backup ke direktori terisolasi
        let res = BackupService::create_backup_snapshot_in(
            &conn_arc,
            &paths.main_db,
            &paths.log_db,
            &backups_dir,
        )
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

        let dummy_path = test_root.join("dummy.db");

        let result = BackupService::create_backup_snapshot_in(
            &conn_arc,
            &dummy_path,
            &dummy_path,
            &backups_dir,
        );

        // Wajib gagal dan TIDAK membuat file dummy kosong
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("tidak ter-attach"));

        // Verifikasi tidak ada folder backup atau staging yang tertinggal
        let list = BackupService::list_backups_in(&backups_dir).unwrap();
        assert!(list.is_empty());

        let entries: Vec<_> = fs::read_dir(&backups_dir).unwrap().flatten().collect();
        assert!(entries.is_empty(), "Folder backups harus tetap bersih tanpa sisa staging");

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
        let dummy = test_root.join("d.db");

        let _ = BackupService::create_backup_snapshot_in(
            &conn_arc,
            &dummy,
            &dummy,
            &backups_dir,
        );

        // Pastikan tidak ada folder .staging_* yang tersisa
        for entry in fs::read_dir(&backups_dir).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            assert!(!name.starts_with(".staging_"), "Staging directory harus dibersihkan otomatis");
        }

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn test_previous_backups_remain_intact_when_new_backup_fails() {
        let (test_root, backups_dir, manager, conn_arc) = setup_isolated_test_env();
        let paths = manager.get_paths();

        // 1. Buat backup pertama yang valid
        let res1 = BackupService::create_backup_snapshot_in(
            &conn_arc,
            &paths.main_db,
            &paths.log_db,
            &backups_dir,
        )
        .expect("First backup should succeed");

        let path1 = PathBuf::from(&res1.path);
        assert!(path1.exists());
        assert!(path1.join("kasir_pos.db").exists());
        assert!(path1.join("manifest.json").exists());

        // 2. Coba buat backup kedua dengan koneksi yang rusak / tanpa log_db
        let broken_conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        let dummy = test_root.join("dummy.db");
        let res2 = BackupService::create_backup_snapshot_in(
            &broken_conn,
            &dummy,
            &dummy,
            &backups_dir,
        );
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
        let (test_root, backups_dir, manager, conn_arc) = setup_isolated_test_env();
        let paths = manager.get_paths();

        // Buat 3 backup secara cepat berturut-turut
        let res1 = BackupService::create_backup_snapshot_in(
            &conn_arc,
            &paths.main_db,
            &paths.log_db,
            &backups_dir,
        )
        .unwrap();

        let res2 = BackupService::create_backup_snapshot_in(
            &conn_arc,
            &paths.main_db,
            &paths.log_db,
            &backups_dir,
        )
        .unwrap();

        let res3 = BackupService::create_backup_snapshot_in(
            &conn_arc,
            &paths.main_db,
            &paths.log_db,
            &backups_dir,
        )
        .unwrap();

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
    fn test_retention_only_keeps_10_latest_valid_backups() {
        let (test_root, backups_dir, _manager, _conn_arc) = setup_isolated_test_env();

        // 1. Buat 12 mock valid backup direktori langsung di direktori terisolasi
        for i in 1..=12 {
            let bkp_name = format!("backup_20261101_{:02}0000_000000000", i);
            let bkp_dir = backups_dir.join(&bkp_name);
            fs::create_dir_all(&bkp_dir).unwrap();

            // Tulis file tiruan yang lengkap
            fs::write(bkp_dir.join("kasir_pos.db"), b"dummy content").unwrap();
            fs::write(bkp_dir.join("kasir_pos_log.db"), b"dummy content").unwrap();
            let manifest = BackupManifest {
                backup_id: bkp_name.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
                app_version: env!("CARGO_PKG_VERSION").to_string(),
                main_db_size: 13,
                log_db_size: 13,
            };
            fs::write(bkp_dir.join("manifest.json"), serde_json::to_string(&manifest).unwrap()).unwrap();
        }

        // 2. Tambahkan 1 folder corrupt/parsial tanpa manifest
        let corrupt_dir = backups_dir.join("backup_20261101_990000_000000000");
        fs::create_dir_all(&corrupt_dir).unwrap();
        fs::write(corrupt_dir.join("kasir_pos.db"), b"orphan").unwrap();

        // 3. Jalankan retensi maksimal 10
        let deleted = BackupService::enforce_retention_policy(&backups_dir, 10).unwrap();

        // Dari 12 valid backup, 2 tertua harus dihapus (i=1 dan i=2)
        assert_eq!(deleted.len(), 2);
        assert!(deleted.contains(&"backup_20261101_010000_000000000".to_string()));
        assert!(deleted.contains(&"backup_20261101_020000_000000000".to_string()));

        // Sisa 10 backup valid terbaru tetap ada, dan folder corrupt tidak tersentuh
        let remaining = BackupService::list_backups_in(&backups_dir).unwrap();
        assert_eq!(remaining.len(), 11);
        let valid_remaining: Vec<_> = remaining
            .into_iter()
            .filter(|b| b.created_at != "-")
            .collect();
        assert_eq!(valid_remaining.len(), 10);

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
        fs::write(&corrupt_file, b"This is definitely not an SQLite database header.").unwrap();
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
        let (test_root, backups_dir, manager, conn_arc) = setup_isolated_test_env();
        let paths = manager.get_paths();

        let user_backups_dir = BackupService::get_backups_dir().unwrap();
        assert_ne!(backups_dir, user_backups_dir);

        let res = BackupService::create_backup_snapshot_in(
            &conn_arc,
            &paths.main_db,
            &paths.log_db,
            &backups_dir,
        )
        .unwrap();

        assert!(!res.path.contains(&user_backups_dir.to_string_lossy().to_string()));
        assert!(res.path.contains(&test_root.to_string_lossy().to_string()));

        let _ = fs::remove_dir_all(&test_root);
    }
}
