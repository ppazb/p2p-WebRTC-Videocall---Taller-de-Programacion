use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FilePathPacket {
    path: Option<PathBuf>, 
    is_server: bool
}


impl FilePathPacket {
    pub fn new(path: Option<PathBuf>, is_server: bool) -> Self {
        FilePathPacket { path, is_server }
    }

    pub fn get_path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn is_server(&self) -> bool {
        self.is_server
    } 
}