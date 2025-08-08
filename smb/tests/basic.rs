//! A basic create file test.

mod common;
use std::str::FromStr;

use common::{TestConstants, TestEnv, make_server_connection};
use serial_test::serial;
use smb::packets::smb2::Status;
use smb::{Client, ClientConfig, UncPath};
use smb::{ConnectionConfig, FileCreateArgs, packets::fscc::FileDispositionInformation};

#[maybe_async::maybe_async]
async fn do_test_basic_integration(
    conn_config: Option<ConnectionConfig>,
    share: Option<&str>,
) -> smb::Result<()> {
    let (mut client, share_path) =
        make_server_connection(share.unwrap_or(TestConstants::DEFAULT_SHARE), conn_config).await?;

    // Create a file
    let file = client
        .create_file(
            &share_path.with_path("basic.txt".to_string()),
            &FileCreateArgs::make_create_new(Default::default(), Default::default()),
        )
        .await?
        .unwrap_file();

    file.set_file_info(FileDispositionInformation::default())
        .await?;

    Ok(())
}

#[test_log::test(maybe_async::test(
    not(feature = "async"),
    async(feature = "async", tokio::test(flavor = "multi_thread"))
))]
#[serial]
async fn test_basic_integration() -> Result<(), Box<dyn std::error::Error>> {
    Ok(do_test_basic_integration(None, None).await?)
}

#[test_log::test(maybe_async::test(
    not(feature = "async"),
    async(feature = "async", tokio::test(flavor = "multi_thread"))
))]
#[serial]
async fn test_basic_netbios() -> Result<(), Box<dyn std::error::Error>> {
    use smb::connection::TransportConfig;

    let conn_config = ConnectionConfig {
        transport: TransportConfig::NetBios,
        ..Default::default()
    };
    Ok(do_test_basic_integration(Some(conn_config), None).await?)
}

#[test_log::test(maybe_async::test(
    not(feature = "async"),
    async(feature = "async", tokio::test(flavor = "multi_thread"))
))]
#[serial]
async fn test_basic_guest() -> smb::Result<()> {
    with_temp_env!(
        [
            (TestEnv::USER, Some(TestEnv::GUEST_USER.to_string())),
            (TestEnv::PASSWORD, Some(TestEnv::GUEST_PASSWORD.to_string())),
        ],
        do_test_basic_integration(
            ConnectionConfig {
                allow_unsigned_guest_access: true,
                ..Default::default()
            }
            .into(),
            Some(TestConstants::PUBLIC_GUEST_SHARE)
        )
    )
}

#[test_log::test(maybe_async::test(
    not(feature = "async"),
    async(feature = "async", tokio::test(flavor = "multi_thread"))
))]
#[serial]
async fn test_basic_auth_fail() -> smb::Result<()> {
    with_temp_env!(
        [(
            TestEnv::PASSWORD,
            Some(TestEnv::DEFAULT_PASSWORD.to_string() + "1")
        ),],
        do_test_basic_auth_fail()
    )
}

#[maybe_async::maybe_async]
async fn do_test_basic_auth_fail() -> smb::Result<()> {
    let res = do_test_basic_integration(None, None).await.unwrap_err();
    match res {
        smb::Error::UnexpectedMessageStatus(status) => {
            assert_eq!(status, Status::LogonFailure as u32);
        }
        _ => panic!("Expected LogonFailure error"),
    }
    smb::Result::Ok(())
}

#[test_log::test(maybe_async::test(
    not(feature = "async"),
    async(feature = "async", tokio::test(flavor = "multi_thread"))
))]
#[serial]
async fn test_connection_timeout_fail() -> Result<(), Box<dyn std::error::Error>> {
    const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
    let mut client = Client::new(ClientConfig {
        connection: ConnectionConfig {
            timeout: Some(CONNECT_TIMEOUT),
            ..Default::default()
        },
        ..Default::default()
    });

    const UNRESPONSIVE_SMB_HOST: &str = "8.8.8.8";
    let share_connect_result = client
        .share_connect(
            &UncPath::from_str(&format!("\\\\{}\\share", UNRESPONSIVE_SMB_HOST)).unwrap(),
            "user",
            "password".to_string(),
        )
        .await;

    if !matches!(
        share_connect_result,
        Err(smb::Error::OperationTimeout(_, CONNECT_TIMEOUT))
    ) {
        return Err("Expected OperationTimeout error!".into());
    }

    Ok(())
}
