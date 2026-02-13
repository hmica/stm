use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum StmError {
    #[error("SSH error: {0}")]
    Ssh(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Tunnel error: {0}")]
    Tunnel(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),
}
