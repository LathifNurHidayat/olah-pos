use crate::database::audit::{AuditAction, AuditLogger};
use crate::database::connection::DbConnection;
use crate::services::crypto::{self, get_or_create_app_key};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidtransConfig {
    pub server_key: String,
    pub client_key: String,
    pub merchant_id: String,
    pub environment: String, // "sandbox" | "production"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleDriveConfig {
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: Option<String>,
    pub backup_folder_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidtransQRISResponse {
    pub order_id: String,
    pub gross_amount: i64,
    pub qr_string: String,
    pub transaction_status: String,
    pub expiry_time: String,
}

pub struct IntegrationService;

impl IntegrationService {
    /// Menyimpan payload konfigurasi layanan eksternal dalam bentuk terenkripsi simetris (AES-256-GCM)
    pub fn save_config(
        db: &DbConnection,
        kode_layanan: &str,
        nama_layanan: &str,
        payload: &serde_json::Value,
        catatan: Option<String>,
        user_id: Option<i64>,
    ) -> std::result::Result<(), String> {
        let key = get_or_create_app_key().map_err(|e| format!("Gagal memuat encryption key: {}", e))?;
        let json_str = payload.to_string();
        let encrypted_payload = crypto::encrypt_string(&json_str, &key)
            .map_err(|e| format!("Gagal mengenkripsi payload konfigurasi: {}", e))?;

        let now = chrono::Utc::now().to_rfc3339();
        let mut conn = db.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| format!("Gagal memulai transaksi integrasi: {}", e))?;

        // Cek apakah konfigurasi sudah ada
        let exists: bool = tx
            .query_row(
                "SELECT COUNT(*) FROM dat_integrasi_config WHERE kode_layanan = ?1;",
                [kode_layanan],
                |r| Ok(r.get::<_, i64>(0)? > 0),
            )
            .unwrap_or(false);

        let config_id: i64 = if exists {
            tx.execute(
                "UPDATE dat_integrasi_config 
                 SET nama_layanan = ?1, payload_terenkripsi = ?2, catatan = ?3, 
                     updated_at = ?4, updated_by = ?5 
                 WHERE kode_layanan = ?6;",
                rusqlite::params![nama_layanan, encrypted_payload, catatan, now, user_id, kode_layanan],
            )
            .map_err(|e| format!("Gagal memperbarui konfigurasi integrasi: {}", e))?;

            tx.query_row(
                "SELECT config_id FROM dat_integrasi_config WHERE kode_layanan = ?1;",
                [kode_layanan],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?
        } else {
            tx.execute(
                "INSERT INTO dat_integrasi_config (
                    created_at, created_by, kode_layanan, nama_layanan, 
                    payload_terenkripsi, catatan
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
                rusqlite::params![now, user_id, kode_layanan, nama_layanan, encrypted_payload, catatan],
            )
            .map_err(|e| format!("Gagal menyimpan konfigurasi integrasi: {}", e))?;

            tx.last_insert_rowid()
        };

        // Audit log dicatat tanpa membocorkan plaintext API keys
        AuditLogger::record(
            &tx,
            "dat_integrasi_config",
            &config_id.to_string(),
            if exists { AuditAction::Update } else { AuditAction::Insert },
            &json!({
                "kode_layanan": kode_layanan,
                "nama_layanan": nama_layanan,
                "status_enkripsi": "AES_256_GCM_ENCRYPTED"
            }),
            user_id,
        )
        .map_err(|e| format!("Gagal mencatat audit log integrasi: {}", e))?;

        tx.commit()
            .map_err(|e| format!("Gagal commit transaksi integrasi: {}", e))?;

        Ok(())
    }

    /// Membaca dan mendekripsi payload konfigurasi layanan tertentu
    pub fn get_config(
        db: &DbConnection,
        kode_layanan: &str,
    ) -> std::result::Result<Option<serde_json::Value>, String> {
        let conn = db.lock().unwrap();
        let encrypted_opt: Option<String> = conn
            .query_row(
                "SELECT payload_terenkripsi FROM dat_integrasi_config WHERE kode_layanan = ?1 AND active_st = 1 AND deleted_st = 0;",
                [kode_layanan],
                |r| r.get(0),
            )
            .ok();

        match encrypted_opt {
            Some(ciphertext) => {
                let key = get_or_create_app_key()
                    .map_err(|e| format!("Gagal memuat encryption key: {}", e))?;
                let decrypted_json = crypto::decrypt_string(&ciphertext, &key)
                    .map_err(|e| format!("Gagal mendekripsi payload konfigurasi: {}", e))?;

                let parsed: serde_json::Value = serde_json::from_str(&decrypted_json)
                    .map_err(|e| format!("Format JSON konfigurasi tidak valid: {}", e))?;

                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    /// Generate payload dynamic QRIS untuk order penjualan
    pub fn generate_qris(
        db: &DbConnection,
        order_id: &str,
        gross_amount: i64,
    ) -> std::result::Result<MidtransQRISResponse, String> {
        // Ambil konfigurasi Midtrans jika ada
        let midtrans_opt = Self::get_config(db, "MIDTRANS_QRIS")?;

        let expiry_time = (chrono::Utc::now() + chrono::Duration::minutes(15)).to_rfc3339();

        // Standard EMVCo QRIS string format simulator / active generator
        let qr_string = format!(
            "00020101021226670014ID.LINKAJA.WWW01189360050300000898740215{}0303UMI51440014ID.MIDTRANS.WWW0215{}520454115303360540{}5802ID5911OLAHPOS6007BANDUNG62170713{}6304ABCD",
            order_id,
            midtrans_opt.as_ref().and_then(|v| v.get("merchant_id")).and_then(|v| v.as_str()).unwrap_or("M1234567"),
            gross_amount,
            order_id
        );

        Ok(MidtransQRISResponse {
            order_id: order_id.to_string(),
            gross_amount,
            qr_string,
            transaction_status: "pending".to_string(),
            expiry_time,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::DbManager;
    use crate::database::migrator::Migrator;

    fn setup_test_db() -> (DbManager, DbConnection) {
        let db_manager = DbManager::init_in_memory().expect("Failed in-memory DB");
        let conn_arc = db_manager.get_connection();
        Migrator::run(&mut conn_arc.lock().unwrap()).expect("Failed running migrations");
        (db_manager, conn_arc)
    }

    #[test]
    fn test_save_and_decrypt_midtrans_config() {
        let (_manager, conn_arc) = setup_test_db();

        let midtrans_payload = json!({
            "server_key": "SB-Mid-server-TEST1234567890",
            "client_key": "SB-Mid-client-0987654321",
            "merchant_id": "G123456789",
            "environment": "sandbox"
        });

        // 1. Simpan konfigurasi
        IntegrationService::save_config(
            &conn_arc,
            "MIDTRANS_QRIS",
            "Midtrans Payment Gateway",
            &midtrans_payload,
            Some("Sandbox key testing".to_string()),
            Some(1),
        )
        .unwrap();

        // 2. Pastikan yang tersimpan di SQLite BUKAN plaintext
        {
            let conn = conn_arc.lock().unwrap();
            let raw_ciphertext: String = conn
                .query_row(
                    "SELECT payload_terenkripsi FROM dat_integrasi_config WHERE kode_layanan = 'MIDTRANS_QRIS';",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(!raw_ciphertext.contains("SB-Mid-server-TEST1234567890"));
            assert!(!raw_ciphertext.contains("sandbox"));
        }

        // 3. Baca kembali dan dekripsi
        let decrypted = IntegrationService::get_config(&conn_arc, "MIDTRANS_QRIS")
            .unwrap()
            .expect("Config should exist");

        assert_eq!(
            decrypted["server_key"].as_str().unwrap(),
            "SB-Mid-server-TEST1234567890"
        );
        assert_eq!(decrypted["environment"].as_str().unwrap(), "sandbox");
    }

    #[test]
    fn test_qris_generation() {
        let (_manager, conn_arc) = setup_test_db();

        let response =
            IntegrationService::generate_qris(&conn_arc, "TRX-TEST-001", 75000).unwrap();
        assert_eq!(response.order_id, "TRX-TEST-001");
        assert_eq!(response.gross_amount, 75000);
        assert!(!response.qr_string.is_empty());
        assert_eq!(response.transaction_status, "pending");
    }
}
