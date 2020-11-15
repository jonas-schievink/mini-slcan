use super::*;
use crate::error::ErrorKind;

fn ok(input: &str, expected: Command) {
    assert_eq!(Command::decode(input.as_bytes()).unwrap(), expected);
}

fn err(input: &str, expected: ErrorKind) {
    assert_eq!(
        Command::decode(input.as_bytes()).unwrap_err().kind(),
        expected,
    );
}

#[test]
fn decode_simple_cmds() {
    ok("O\r", Command::Open);
    ok("C\r", Command::Close);
    err("C\n", ErrorKind::Decode);
    err("C", ErrorKind::Eof);
    err("", ErrorKind::Eof);
}

#[test]
fn decode_setup_cmds() {
    ok(
        "S0\r",
        Command::SetupWithBitrate {
            bitrate: Bitrate::_10kbit,
        },
    );
    ok(
        "S8\r",
        Command::SetupWithBitrate {
            bitrate: Bitrate::_1mbit,
        },
    );

    err("S9\r", ErrorKind::Decode);
}

#[test]
fn decode_tx_cmds() {
    ok(
        "t7FF0\r",
        Command::TxStandard {
            identifier: Identifier::from_raw(0x7FF).unwrap(),
            frame: [].into(),
        },
    );
    ok(
        "t0000\r",
        Command::TxStandard {
            identifier: Identifier::from_raw(0).unwrap(),
            frame: [].into(),
        },
    );
    ok(
        "t7FF1AA\r",
        Command::TxStandard {
            identifier: Identifier::from_raw(0x7FF).unwrap(),
            frame: [0xAA].into(),
        },
    );
    err("t8000\r", ErrorKind::Decode);
    err("t800\r", ErrorKind::Decode);
    err("t80\r", ErrorKind::Decode);

    ok(
        "T1FFFFFFF80001020304050607\r",
        Command::TxExt {
            identifier: ExtIdentifier::from_raw(0x1FFF_FFFF).unwrap(),
            frame: [0, 1, 2, 3, 4, 5, 6, 7].into(),
        },
    );
    ok(
        "T1111111184142434445464748\r",
        Command::TxExt {
            identifier: ExtIdentifier::from_raw(0x1111_1111).unwrap(),
            frame: (*b"ABCDEFGH").into(),
        },
    );

    ok(
        "R1FFFFFFF8\r",
        Command::TxExtRtr {
            identifier: ExtIdentifier::from_raw(0x1FFF_FFFF).unwrap(),
            len: 8,
        },
    );

    err("R1FFFFFFF9\r", ErrorKind::Decode);
}

#[test]
fn mismatched_len() {
    err("t7FF1\r", ErrorKind::Decode);
    err("t7FF1AABB\r", ErrorKind::Decode);
}

fn cmdbuf_decode(chunks: &[&[u8]], res: &[Result<Command, ErrorKind>]) {
    let mut expected = res.iter();
    let mut buf = CommandBuf::new();
    for chunk in chunks {
        buf.tail_mut()[..chunk.len()].copy_from_slice(chunk);

        for res in buf.advance_by(chunk.len() as u8) {
            let exp = expected.next().expect("too few expected results");
            let res = res.map_err(|e| e.kind());
            assert_eq!(&res, exp);
        }
    }
}

#[test]
fn cmdbuf() {
    cmdbuf_decode(
        &[b"T1111111184142434445464748\r"],
        &[Ok(Command::TxExt {
            identifier: ExtIdentifier::from_raw(0x1111_1111).unwrap(),
            frame: (*b"ABCDEFGH").into(),
        })],
    );
    cmdbuf_decode(
        &[b"T11111111841424344454", b"64748", b"\r"],
        &[Ok(Command::TxExt {
            identifier: ExtIdentifier::from_raw(0x1111_1111).unwrap(),
            frame: (*b"ABCDEFGH").into(),
        })],
    );
    cmdbuf_decode(
        &[b"T11111111841424344454", b"64748", b"\r", b"S0\r"],
        &[
            Ok(Command::TxExt {
                identifier: ExtIdentifier::from_raw(0x1111_1111).unwrap(),
                frame: (*b"ABCDEFGH").into(),
            }),
            Ok(Command::SetupWithBitrate {
                bitrate: Bitrate::_10kbit,
            }),
        ],
    );
    cmdbuf_decode(
        &[b"S0\rS0"],
        &[Ok(Command::SetupWithBitrate {
            bitrate: Bitrate::_10kbit,
        })],
    );
    cmdbuf_decode(
        &[b"\rS0\r"],
        &[
            Err(ErrorKind::Decode),
            Ok(Command::SetupWithBitrate {
                bitrate: Bitrate::_10kbit,
            }),
        ],
    );
    cmdbuf_decode(
        &[b"INVALID\rS0", b"\r"],
        &[
            Err(ErrorKind::Decode),
            Ok(Command::SetupWithBitrate {
                bitrate: Bitrate::_10kbit,
            }),
        ],
    );
}
