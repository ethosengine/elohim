//! Windows directory custody requires both a flush-capable handle and NTFS.
//!
//! MS-FSA 2.1.5.7 specifies directory metadata and physical-storage flushing;
//! its product-behavior note 80 says other filesystems may return success without
//! persisting the directory structure. Never interpret that success as custody.
//! https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fsa/0de7dc40-9627-437e-a4df-c4696cdc3d02
//! https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fsa/4e3695bd-7574-4f24-a223-b4679c065b63

use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use std::ptr::null_mut;

use windows_sys::Win32::Storage::FileSystem::{
    GetVolumeInformationByHandleW, FILE_FLAG_BACKUP_SEMANTICS,
};

pub(super) fn open_sync_directory(path: &Path) -> io::Result<File> {
    // CreateFileW needs BACKUP_SEMANTICS for directory handles; sync_all maps
    // to FlushFileBuffers, which requires write access. No privilege is enabled.
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?;
    if !file.metadata()?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "custody path is not a directory",
        ));
    }
    // Windows MAX_PATH + terminator.
    let mut filesystem = [0_u16; 261];
    // SAFETY: file owns a live handle throughout this call. The only non-null
    // output points to the writable array, with its exact capacity in WCHARs.
    let queried = unsafe {
        GetVolumeInformationByHandleW(
            file.as_raw_handle(),
            null_mut(),
            0,
            null_mut(),
            null_mut(),
            null_mut(),
            filesystem.as_mut_ptr(),
            filesystem.len() as u32,
        )
    };
    if queried == 0 {
        return Err(io::Error::last_os_error());
    }
    require_ntfs(&filesystem)?;
    Ok(file)
}

fn require_ntfs(filesystem: &[u16]) -> io::Result<()> {
    let end = filesystem
        .iter()
        .position(|unit| *unit == 0)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "unterminated Windows filesystem name",
            )
        })?;
    let name = &filesystem[..end];
    if !"NTFS".encode_utf16().eq(name.iter().copied()) {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "durable directory custody requires NTFS, found {}",
                String::from_utf16_lossy(name)
            ),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filesystems_without_the_directory_flush_contract_refuse() {
        for filesystem in ["FAT", "FAT32", "exFAT", "ReFS", "", "unknown"] {
            let name: Vec<_> = filesystem.encode_utf16().chain([0]).collect();
            assert_eq!(
                require_ntfs(&name).unwrap_err().kind(),
                io::ErrorKind::Unsupported
            );
        }
        assert_eq!(
            require_ntfs(&[78, 84, 70, 83]).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        require_ntfs(&"NTFS".encode_utf16().chain([0]).collect::<Vec<_>>()).unwrap();
    }
}
