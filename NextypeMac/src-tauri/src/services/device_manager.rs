// ============================================================
// 设备管理和配对码服务
// ============================================================

use parking_lot::RwLock;
use rand::Rng;
use std::time::{Duration, Instant};

/// 配对码信息
#[derive(Debug, Clone)]
pub struct PairingCodeInfo {
    pub code: String,
    pub created_at: Instant,
    pub expires_in: Duration,
}

impl PairingCodeInfo {
    /// 检查配对码是否已过期
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.expires_in
    }

    /// 获取剩余有效时间（秒）
    pub fn remaining_seconds(&self) -> u64 {
        if self.is_expired() {
            0
        } else {
            (self.expires_in - self.created_at.elapsed()).as_secs()
        }
    }
}

/// 设备管理器
pub struct DeviceManager {
    current_pairing_code: RwLock<Option<PairingCodeInfo>>,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            current_pairing_code: RwLock::new(None),
        }
    }

    /// 生成易记忆的配对码模式
    fn generate_memorable_patterns() -> Vec<String> {
        let mut patterns = Vec::new();

        // 模式1: AABB (如 1122, 3344, 7788)
        for a in 1..=9 {
            for b in 0..=9 {
                if a != b {
                    patterns.push(format!("{}{}{}{}", a, a, b, b));
                }
            }
        }

        // 模式2: ABAB (如 1212, 3434, 1919)
        for a in 1..=9 {
            for b in 0..=9 {
                if a != b {
                    patterns.push(format!("{}{}{}{}", a, b, a, b));
                }
            }
        }

        // 模式3: ABCD 连续递增 (如 1234, 2345, 4567)
        for start in 1..=6 {
            patterns.push(format!(
                "{}{}{}{}",
                start,
                start + 1,
                start + 2,
                start + 3
            ));
        }

        // 模式4: DCBA 连续递减 (如 4321, 9876, 5432)
        for start in (4..=9).rev() {
            patterns.push(format!(
                "{}{}{}{}",
                start,
                start - 1,
                start - 2,
                start - 3
            ));
        }

        // 模式5: ABBA 回文 (如 1221, 3443, 9889)
        for a in 1..=9 {
            for b in 0..=9 {
                if a != b {
                    patterns.push(format!("{}{}{}{}", a, b, b, a));
                }
            }
        }

        // 模式6: AAAB / ABBB (如 1112, 3888)
        for a in 1..=9 {
            for b in 0..=9 {
                if a != b {
                    patterns.push(format!("{}{}{}{}", a, a, a, b)); // AAAB
                    if b != 0 {
                        patterns.push(format!("{}{}{}{}", a, b, b, b)); // ABBB
                    }
                }
            }
        }

        // 模式7: AABA / ABAA (如 1121, 1211)
        for a in 1..=9 {
            for b in 0..=9 {
                if a != b {
                    patterns.push(format!("{}{}{}{}", a, a, b, a)); // AABA
                    patterns.push(format!("{}{}{}{}", a, b, a, a)); // ABAA
                }
            }
        }

        patterns
    }

    /// 生成配对码
    ///
    /// # Arguments
    /// * `force_random` - 是否强制使用随机模式（用于重试时）
    pub fn generate_pairing_code(&self, force_random: bool) -> String {
        let code = if force_random {
            // 强制随机模式
            let mut rng = rand::thread_rng();
            format!("{:04}", rng.gen_range(1000..10000))
        } else {
            // 优先使用易记忆模式
            let patterns = Self::generate_memorable_patterns();
            let mut rng = rand::thread_rng();
            let random_index = rng.gen_range(0..patterns.len());
            patterns[random_index].clone()
        };

        // 存储配对码（60秒有效）
        *self.current_pairing_code.write() = Some(PairingCodeInfo {
            code: code.clone(),
            created_at: Instant::now(),
            expires_in: Duration::from_secs(60),
        });

        let mode_desc = if force_random { "随机" } else { "易记忆" };
        tracing::info!("🔑 生成配对码: {} ({}模式, 60秒内有效)", code, mode_desc);

        code
    }

    /// 验证配对码
    pub fn verify_pairing_code(&self, code: &str) -> bool {
        let mut pairing_code = self.current_pairing_code.write();

        match &*pairing_code {
            None => {
                tracing::warn!("❌ 没有活跃的配对码");
                false
            }
            Some(info) => {
                if info.is_expired() {
                    tracing::warn!("❌ 配对码已过期");
                    *pairing_code = None;
                    false
                } else if info.code == code {
                    tracing::info!("✅ 配对码验证成功");
                    // 验证成功后清除配对码
                    *pairing_code = None;
                    true
                } else {
                    tracing::warn!("❌ 配对码不匹配");
                    false
                }
            }
        }
    }

    /// 获取当前配对码信息
    pub fn get_current_pairing_code(&self) -> Option<(String, u64)> {
        let pairing_code = self.current_pairing_code.read();

        pairing_code.as_ref().and_then(|info| {
            if info.is_expired() {
                None
            } else {
                Some((info.code.clone(), info.remaining_seconds()))
            }
        })
    }

    /// 清除当前配对码
    pub fn clear_pairing_code(&self) {
        *self.current_pairing_code.write() = None;
    }

    /// 生成加密密钥
    pub fn generate_encryption_key() -> String {
        let mut key = [0u8; 32];
        rand::thread_rng().fill(&mut key);
        hex::encode(key)
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 生成二维码数据 URL
pub fn generate_qr_code_data_url(content: &str) -> Result<String, String> {
    use base64::Engine;
    use image::ImageEncoder;
    use qrcode::QrCode;

    let code = QrCode::new(content.as_bytes()).map_err(|e| e.to_string())?;
    let image = code.render::<image::Luma<u8>>().build();

    // 转换为 PNG 数据
    let mut png_data = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_data);
    encoder
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::L8,
        )
        .map_err(|e| e.to_string())?;

    // 转换为 base64 data URL
    let base64_data = base64::engine::general_purpose::STANDARD.encode(&png_data);
    Ok(format!("data:image/png;base64,{}", base64_data))
}
