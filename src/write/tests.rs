use super::*;

fn enc_response(response: Response, expected: &[u8]) {
    let mut buf = ResponseBuf::new();
    let buf = response.encode(&mut buf).unwrap();
    assert_eq!(expected, buf);
}

fn enc_notif(notif: Notification, expected: &[u8]) {
    let mut buf = NotificationBuf::new();
    let buf = notif.encode(&mut buf).unwrap();
    assert_eq!(expected, buf);
}

#[test]
fn encode_simple_responses() {
    enc_response(Response::Error, b"\x07");
    enc_response(Response::Ack, b"\r");
    enc_response(Response::TxAck, b"z\r");
    enc_response(Response::ExtTxAck, b"Z\r");
}

#[test]
fn encode_status() {
    enc_response(Response::Status(Status::RX_FIFO_FULL), b"F01\r");
}

#[test]
fn encode_version() {
    enc_response(
        Response::Version {
            hardware_version: 1,
            software_version: 2,
        },
        b"V0102\r",
    );
}

#[test]
fn encode_serial() {
    enc_response(
        Response::Serial(SerialNumber::new(*b"TEST").unwrap()),
        b"NTEST\r",
    );
}

#[test]
fn encode_notifs() {
    enc_notif(
        Notification::Rx {
            identifier: Identifier::from_raw(0x100).unwrap(),
            frame: [0x11, 0x33].into(),
        },
        b"t10021133",
    );
    enc_notif(
        Notification::Rx {
            identifier: Identifier::from_raw(0x7FF).unwrap(),
            frame: [].into(),
        },
        b"t7FF0",
    );
    enc_notif(
        Notification::RxExtRtr {
            identifier: ExtIdentifier::from_raw(0).unwrap(),
            len: 5,
        },
        b"R000000005",
    );
}
