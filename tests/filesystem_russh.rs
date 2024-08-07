use std::ffi::OsString;

use common::{entries_contain, gen_nested_tmp_path, gen_tmp_path, RusshData};
use remoteify::filesystem::{LinuxFileMetadata, LinuxFileType, LinuxFilesystem, LinuxOpenOptions, LinuxPermissions};
use russh_sftp::protocol::FileAttributes;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod common;

#[tokio::test]
async fn exists_is_false_for_missing_item() {
    let test_data = RusshData::setup().await;
    let path = gen_tmp_path();
    assert!(!test_data.implementation.exists(&path).await.expect("Call failed"));
}

#[tokio::test]
async fn exists_is_true_for_existent_file() {
    let test_data = RusshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.create(path.to_string_lossy()).await.unwrap();
    assert!(test_data.implementation.exists(&path).await.expect("Call failed"));
}

#[tokio::test]
async fn exists_is_true_for_existent_dir() {
    let test_data = RusshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.create_dir(path.to_string_lossy()).await.unwrap();
    assert!(test_data.implementation.exists(&path).await.expect("Call failed"));
}

#[tokio::test]
async fn open_file_with_read_should_work() {
    let test_data = RusshData::setup().await;
    let path = test_data.init_file("content").await;
    let mut handle = test_data
        .implementation
        .open_file(&path, LinuxOpenOptions::new().read())
        .await
        .expect("Call failed");
    let mut buf = String::new();
    handle.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "content");
}

#[tokio::test]
async fn open_file_with_write_should_work() {
    let test_data = RusshData::setup().await;
    let path = test_data.init_file("content").await;
    let mut handle = test_data
        .implementation
        .open_file(&path, LinuxOpenOptions::new().write())
        .await
        .expect("Call failed");
    handle.write_all(b"CON").await.unwrap();
    test_data.assert_file(&path, "CONtent").await;
}

#[tokio::test]
async fn open_file_with_append_should_work() {
    let test_data = RusshData::setup().await;
    let path = test_data.init_file("first").await;
    let mut handle = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().write().append())
        .await
        .expect("Call failed");
    handle.write_all(b"second").await.unwrap();
    test_data.assert_file(&path, "firstsecond").await;
}

#[tokio::test]
async fn open_file_with_truncate_should_work() {
    let test_data = RusshData::setup().await;
    let path = test_data.init_file("current").await;
    let mut handle = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().write().truncate())
        .await
        .expect("Call failed");
    handle.write_all(b"next").await.unwrap();
    test_data.assert_file(&path, "next").await;
}

#[tokio::test]
async fn open_file_with_create_should_work() {
    let test_data = RusshData::setup().await;
    let path = gen_tmp_path();
    let mut handle = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().write().create())
        .await
        .expect("Call failed");
    handle.write_all(b"content").await.unwrap();
    test_data.assert_file(&path, "content").await;
}

#[tokio::test]
async fn create_file_should_persist() {
    let test_data = RusshData::setup().await;
    let path = gen_tmp_path();
    test_data.implementation.create_file(&path).await.expect("Call failed");
    assert!(test_data.sftp.try_exists(path.to_string_lossy()).await.unwrap());
}

#[tokio::test]
async fn rename_file_should_perform_change() {
    let test_data = RusshData::setup().await;
    let old_path = test_data.init_file("some content").await;
    let new_path = gen_tmp_path();
    test_data
        .implementation
        .rename_file(&old_path, &new_path)
        .await
        .expect("Call failed");
    assert!(!test_data.sftp.try_exists(old_path.to_string_lossy()).await.unwrap());
    assert!(test_data.sftp.try_exists(new_path.to_string_lossy()).await.unwrap());
    test_data.assert_file(&new_path, "some content").await;
}

#[tokio::test]
async fn copy_file_should_perform_change() {
    let test_data = RusshData::setup().await;
    let old_path = test_data.init_file("content").await;
    let new_path = gen_tmp_path();
    test_data
        .implementation
        .copy_file(&old_path, &new_path)
        .await
        .expect("Call failed");
    test_data.assert_file(&old_path, "content").await;
    test_data.assert_file(&new_path, "content").await;
}

#[tokio::test]
async fn canonicalize_should_perform_transformation() {
    let test_data = RusshData::setup().await;
    assert_eq!(
        test_data
            .implementation
            .canonicalize(&OsString::from("/tmp/../tmp/../tmp"))
            .await
            .expect("Call failed")
            .to_str()
            .unwrap(),
        "/tmp"
    );
}

#[tokio::test]
async fn symlink_should_perform_linking() {
    let test_data = RusshData::setup().await;
    let src_path = test_data.init_file("content").await;
    let dst_path = gen_tmp_path();
    test_data
        .implementation
        .create_symlink(&src_path, &dst_path)
        .await
        .expect("Call failed");
    assert_eq!(
        test_data.sftp.read_link(dst_path.to_string_lossy()).await.unwrap(),
        src_path.to_str().unwrap()
    );
}

#[tokio::test]
async fn hardlink_should_perform_linking() {
    let test_data = RusshData::setup().await;
    let src_path = test_data.init_file("content").await;
    let dst_path = gen_tmp_path();
    test_data
        .implementation
        .create_hard_link(&src_path, &dst_path)
        .await
        .expect("Call failed");
    test_data.assert_file(&src_path, "content").await;
    test_data.assert_file(&dst_path, "content").await;
}

#[tokio::test]
async fn read_link_should_return_correct_source_path() {
    let test_data = RusshData::setup().await;
    let src_path = test_data.init_file("content").await;
    let dst_path = gen_tmp_path();
    test_data
        .sftp
        .symlink(src_path.to_string_lossy(), dst_path.to_string_lossy())
        .await
        .unwrap();
    assert_eq!(
        test_data
            .implementation
            .read_link(&dst_path)
            .await
            .expect("Call failed"),
        src_path
    );
}

#[tokio::test]
async fn set_permissions_should_perform_change() {
    let test_data = RusshData::setup().await;
    let path = test_data.init_file("content").await;
    test_data
        .implementation
        .set_permissions(&path, LinuxPermissions::from_bits(777).unwrap())
        .await
        .expect("Call failed");
    assert_eq!(
        test_data
            .sftp
            .metadata(path.to_string_lossy())
            .await
            .unwrap()
            .permissions
            .unwrap(),
        33545
    );
}

#[tokio::test]
async fn remove_file_should_persist_changes() {
    let test_data = RusshData::setup().await;
    let path = test_data.init_file("content").await;
    test_data.implementation.remove_file(&path).await.expect("Call failed");
    assert!(!test_data.sftp.try_exists(path.to_string_lossy()).await.unwrap());
}

#[tokio::test]
async fn create_dir_should_persist() {
    let test_data = RusshData::setup().await;
    let path = gen_tmp_path();
    test_data.implementation.create_dir(&path).await.expect("Call failed");
    assert!(test_data.sftp.try_exists(path.to_string_lossy()).await.unwrap());
}

#[tokio::test]
async fn create_dir_recursively_should_persist() {
    let test_data = RusshData::setup().await;
    let (_, child_path) = gen_nested_tmp_path();
    test_data
        .implementation
        .create_dir_recursively(&child_path)
        .await
        .expect("Call failed");
    assert!(test_data.sftp.try_exists(child_path.to_string_lossy()).await.unwrap());
}

#[tokio::test]
async fn list_dir_returns_correct_results() {
    let test_data = RusshData::setup().await;
    let file_path = test_data.init_file("content").await;
    let dir_path = gen_tmp_path();
    test_data.sftp.create_dir(dir_path.to_string_lossy()).await.unwrap();
    let symlink_path = gen_tmp_path();
    test_data
        .sftp
        .symlink(file_path.to_string_lossy(), symlink_path.to_string_lossy())
        .await
        .unwrap();

    let entries = test_data
        .implementation
        .list_dir(&OsString::from("/tmp"))
        .await
        .expect("Call failed");

    entries_contain(&entries, LinuxFileType::File, &file_path);
    entries_contain(&entries, LinuxFileType::Dir, &dir_path);
    entries_contain(&entries, LinuxFileType::Symlink, &symlink_path);
}

#[tokio::test]
async fn remove_dir_should_persist() {
    let test_data = RusshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.create_dir(path.to_string_lossy()).await.unwrap();
    test_data.implementation.remove_dir(&path).await.expect("Call failed");
    assert!(!test_data.sftp.try_exists(path.to_string_lossy()).await.unwrap());
}

#[tokio::test]
async fn remove_dir_recursively_should_persist() {
    let test_data = RusshData::setup().await;
    let (parent_path, child_path) = gen_nested_tmp_path();
    test_data.sftp.create_dir(parent_path.to_string_lossy()).await.unwrap();
    test_data.sftp.create_dir(child_path.to_string_lossy()).await.unwrap();
    test_data
        .implementation
        .remove_dir_recursively(&parent_path)
        .await
        .expect("Call failed");
    assert!(!test_data.sftp.try_exists(parent_path.to_string_lossy()).await.unwrap());
}

#[tokio::test]
async fn get_metadata_should_return_correct_result() {
    let test_data = RusshData::setup().await;
    let path = test_data.init_file("content").await;
    let expected_metadata = test_data.sftp.metadata(path.to_string_lossy()).await.unwrap();
    let actual_metadata = test_data.implementation.get_metadata(&path).await.expect("Call failed");
    assert_metadata(expected_metadata, actual_metadata, LinuxFileType::File);
}

#[tokio::test]
async fn get_symlink_metadata_should_return_correct_result() {
    let test_data = RusshData::setup().await;
    let src_path = test_data.init_file("content").await;
    let symlink_path = gen_tmp_path();
    test_data
        .sftp
        .symlink(src_path.to_string_lossy(), symlink_path.to_string_lossy())
        .await
        .unwrap();
    let expected_metadata = test_data
        .sftp
        .symlink_metadata(symlink_path.to_string_lossy())
        .await
        .unwrap();
    let actual_metadata = test_data
        .implementation
        .get_symlink_metadata(&symlink_path)
        .await
        .unwrap();
    assert_metadata(expected_metadata, actual_metadata, LinuxFileType::Symlink);
}

fn assert_metadata(expected_metadata: FileAttributes, actual_metadata: LinuxFileMetadata, _file_type: LinuxFileType) {
    assert!(matches!(actual_metadata.file_type.unwrap(), _file_type));
    assert_eq!(actual_metadata.size.unwrap(), expected_metadata.size.unwrap());
    assert_eq!(
        actual_metadata.permissions.unwrap().bits(),
        expected_metadata.permissions.unwrap()
    );
    assert_eq!(
        actual_metadata.modified_time.unwrap(),
        expected_metadata.modified().unwrap()
    );
    assert_eq!(
        actual_metadata.accessed_time.unwrap(),
        expected_metadata.modified().unwrap()
    );
    assert_eq!(actual_metadata.created_time, None);
    assert_eq!(actual_metadata.user_id.unwrap(), expected_metadata.uid.unwrap());
    assert_eq!(actual_metadata.user_name, None);
    assert_eq!(actual_metadata.group_id.unwrap(), expected_metadata.gid.unwrap());
    assert_eq!(actual_metadata.group_name, None);
}
