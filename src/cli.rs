use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "md-prune-image")]
#[command(about = "Remove orphaned image files from markdown directories", long_about = None)]
#[command(styles = clap_cargo::style::CLAP_STYLING)]
pub struct Cli {
    #[arg(value_name = "DIRECTORY")]
    pub directory: PathBuf,

    #[arg(long, group = "action")]
    pub recycle: bool,

    #[arg(long, group = "action")]
    pub delete: bool,

    #[arg(long, group = "action", value_name = "DIR")]
    pub r#move: Option<PathBuf>,

    #[arg(long, default_value = "jpg,jpeg,png,gif,bmp,svg,webp")]
    pub extensions: String,
}

#[derive(Debug, Clone)]
pub enum Action {
    Delete,
    Recycle,
    Move(PathBuf),
}

impl Cli {
    pub fn action(&self) -> Action {
        if self.delete {
            Action::Delete
        } else if let Some(ref dir) = self.r#move {
            Action::Move(dir.clone())
        } else {
            Action::Recycle
        }
    }
}
