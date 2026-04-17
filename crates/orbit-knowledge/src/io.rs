use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineEnding {
    Lf,
    CrLf,
}

impl LineEnding {
    pub(crate) fn detect(content: &str) -> Self {
        let bytes = content.as_bytes();
        let mut index = 0usize;
        while index < bytes.len() {
            match bytes[index] {
                b'\n' => return Self::Lf,
                b'\r' => {
                    if bytes.get(index + 1) == Some(&b'\n') {
                        return Self::CrLf;
                    }
                }
                _ => {}
            }
            index += 1;
        }
        Self::Lf
    }

    pub(crate) fn separator(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }
}

pub(crate) fn render_content(
    lines: Vec<String>,
    line_ending: LineEnding,
    preserve_trailing_newline: bool,
) -> String {
    let separator = line_ending.separator();
    let mut content = lines.join(separator);
    if preserve_trailing_newline && !content.is_empty() {
        content.push_str(separator);
    } else if preserve_trailing_newline && content.is_empty() {
        content.push_str(separator);
    }
    content
}

pub(crate) fn write_text_atomic_durable(path: &Path, content: &str) -> io::Result<()> {
    let mut staged = StagedTextFile::new(path, content)?;
    staged.commit()
}

pub(crate) fn write_text_atomic(path: &Path, content: &str) -> io::Result<()> {
    let mut staged = StagedTextFile::new_volatile(path, content)?;
    staged.commit()
}

pub(crate) struct StagedTextFile {
    target_path: PathBuf,
    temp_path: PathBuf,
    sync_parent: bool,
    committed: bool,
}

impl StagedTextFile {
    pub(crate) fn new(target_path: &Path, content: &str) -> io::Result<Self> {
        Self::new_internal(target_path, content, true)
    }

    pub(crate) fn new_volatile(target_path: &Path, content: &str) -> io::Result<Self> {
        Self::new_internal(target_path, content, false)
    }

    fn new_internal(target_path: &Path, content: &str, durable: bool) -> io::Result<Self> {
        let parent = target_path.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("no parent dir for {}", target_path.display()),
            )
        })?;
        fs::create_dir_all(parent)?;

        let temp_path = temp_path_for(target_path);
        let mut file = OpenOptions::new()
            .create_new(true)
            .truncate(true)
            .write(true)
            .open(&temp_path)?;

        if let Ok(metadata) = fs::metadata(target_path) {
            fs::set_permissions(&temp_path, metadata.permissions())?;
        }

        file.write_all(content.as_bytes())?;
        if durable {
            file.sync_all()?;
        }
        drop(file);

        Ok(Self {
            target_path: target_path.to_path_buf(),
            temp_path,
            sync_parent: durable,
            committed: false,
        })
    }

    pub(crate) fn commit(&mut self) -> io::Result<()> {
        fs::rename(&self.temp_path, &self.target_path)?;
        self.committed = true;
        if self.sync_parent {
            sync_parent_dir(&self.target_path)?;
        }
        Ok(())
    }
}

impl Drop for StagedTextFile {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        let _ = fs::remove_file(&self.temp_path);
    }
}

fn temp_path_for(target_path: &Path) -> PathBuf {
    let file_name = target_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("orbit");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_name = format!(".{file_name}.{nanos}.{counter}.tmp");
    target_path.with_file_name(temp_name)
}

fn sync_parent_dir(target_path: &Path) -> io::Result<()> {
    let parent = target_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("no parent dir for {}", target_path.display()),
        )
    })?;
    File::open(parent)?.sync_all()
}
