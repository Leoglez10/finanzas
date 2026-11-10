use std::path::{Path, PathBuf};
use super::ImportError;

pub fn extract_actual(path: &Path, scratch: &Path) -> Result<PathBuf, ImportError> {
    use std::io::{Read, Seek};
    let mut file = std::fs::File::open(path)?;
    let mut header = [0; 16];
    if file.read_exact(&mut header).is_ok() && &header == b"SQLite format 3\0" {
        return Ok(path.to_owned());
    }
    file.rewind()?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut entry = archive.by_name("db.sqlite")?;
    let output = scratch.join("db.sqlite");
    let mut destination = std::fs::OpenOptions::new().write(true).create_new(true).open(&output)?;
    std::io::copy(&mut entry, &mut destination)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, time::{SystemTime, UNIX_EPOCH}};

    fn scratch() -> PathBuf {
        let path = std::env::temp_dir().join(format!("actual-extract-{}-{}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn extract_actual_raw_sqlite() {
        let dir = scratch();
        let path = dir.join("source.sqlite");
        std::fs::write(&path, b"SQLite format 3\0test").unwrap();
        assert_eq!(extract_actual(&path, &dir).unwrap(), path);
    }

    #[test]
    fn extract_actual_zip() {
        let dir = scratch();
        let path = dir.join("source.zip");
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("db.sqlite", zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(b"SQLite format 3\0test").unwrap();
        zip.finish().unwrap();
        let extracted = extract_actual(&path, &dir).unwrap();
        assert_eq!(extracted, dir.join("db.sqlite"));
        assert_eq!(std::fs::read(extracted).unwrap(), b"SQLite format 3\0test");
    }
}
