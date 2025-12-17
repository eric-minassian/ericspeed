#[derive(Debug, Clone)]
pub struct Settings {
    pub ping_count: usize,
    pub download_size_mb: u64,
    pub upload_size_mb: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ping_count: 30,
            download_size_mb: 100,
            upload_size_mb: 50,
        }
    }
}

impl Settings {
    pub fn download_size_bytes(&self) -> u64 {
        self.download_size_mb * 1_000_000
    }

    pub fn upload_size_bytes(&self) -> usize {
        (self.upload_size_mb * 1_000_000) as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    PingCount,
    DownloadSize,
    UploadSize,
}

impl SettingsField {
    pub fn next(self) -> Self {
        match self {
            SettingsField::PingCount => SettingsField::DownloadSize,
            SettingsField::DownloadSize => SettingsField::UploadSize,
            SettingsField::UploadSize => SettingsField::PingCount,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            SettingsField::PingCount => SettingsField::UploadSize,
            SettingsField::DownloadSize => SettingsField::PingCount,
            SettingsField::UploadSize => SettingsField::DownloadSize,
        }
    }
}
