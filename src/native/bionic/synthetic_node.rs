use std::sync::Arc;

pub const PROC_DEVICE: (u32, u32) = (0, 4);
pub const SYS_DEVICE: (u32, u32) = (0, 0x1c);

pub type FileContent = Arc<dyn Fn() -> Vec<u8> + Send + Sync>;
pub type DirContent = Arc<dyn Fn() -> Vec<(String, u8)> + Send + Sync>;
pub type LinkContent = Arc<dyn Fn() -> String + Send + Sync>;

#[derive(Clone)]
pub enum Synthetic {
    File { content: FileContent, device: (u32, u32) },
    Dir { entries: DirContent, device: (u32, u32) },
    Link { target: LinkContent, device: (u32, u32) },
}

impl Synthetic {
    pub fn file(device: (u32, u32), content: impl Fn() -> Vec<u8> + Send + Sync + 'static) -> Self {
        Self::File { content: Arc::new(content), device }
    }

    pub fn dir(device: (u32, u32), entries: impl Fn() -> Vec<(String, u8)> + Send + Sync + 'static) -> Self {
        Self::Dir { entries: Arc::new(entries), device }
    }

    pub fn link(device: (u32, u32), target: impl Fn() -> String + Send + Sync + 'static) -> Self {
        Self::Link { target: Arc::new(target), device }
    }

    pub fn device(&self) -> (u32, u32) {
        match self {
            Self::File { device, .. } | Self::Dir { device, .. } | Self::Link { device, .. } => *device,
        }
    }
}
