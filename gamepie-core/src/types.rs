#[derive(Debug, Clone)]
pub struct RetroSystemInfo {
    pub library_name: String,
    pub library_version: String,
    pub valid_extensions: String,
    pub need_fullpath: bool,
    pub block_extract: bool,
}

#[derive(Clone)]
pub struct CoreInfo {
    path: String,
    sys_info: RetroSystemInfo,
    extensions: Vec<String>,
}

impl CoreInfo {
    pub fn new(path: std::fs::DirEntry, sys_info: RetroSystemInfo) -> Self {
        let extensions = sys_info
            .valid_extensions
            .split('|')
            .map(|s| s.to_owned())
            .collect();
        CoreInfo {
            path: path
                .path()
                .to_str()
                .expect("Path not valid UTF-8")
                .to_owned(),
            sys_info,
            extensions,
        }
    }

    pub fn extensions_str(&self) -> String {
        self.extensions.join(", ")
    }

    pub fn supports(&self, ext: &str) -> bool {
        self.extensions.iter().any(|s| s == ext)
    }

    pub fn name(&self) -> String {
        self.sys_info.library_name.clone()
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn sys_info(&self) -> &RetroSystemInfo {
        &self.sys_info
    }
}
