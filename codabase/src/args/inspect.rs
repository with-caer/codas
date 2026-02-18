use codas::{
    codec::{CodecError, DataHeader, ReadsDecodable, TEMP_BUFFER_SIZE},
    stream::Reads,
    types::binary::hex_from_bytes,
};

use super::{open_file_or_stdin, InspectCommand};

/// Executes `command` locally.
pub fn execute_inspect_command(command: InspectCommand) {
    // Open input source.
    let mut bytes = open_file_or_stdin(command.source).expect("source doesn't exist");
    let mut buffer = Vec::with_capacity(TEMP_BUFFER_SIZE);
    bytes.read_to_end(&mut buffer).expect("source read failed");

    // Inspect the data.
    inspect_data(&mut buffer.as_slice(), 0).unwrap();
}

fn inspect_data(data: &mut impl Reads, depth: usize) -> Result<(), CodecError> {
    // Decode header.
    let header: DataHeader = data.read_data()?;
    let format = header.format;
    let count = header.count;

    eprint!("|-");
    for _ in 0..depth {
        eprint!("-");
    }

    if format.ordinal != 0 {
        eprintln!(
            " {} O({}) - {} Bytes, {} Data",
            count, format.ordinal, format.blob_size, format.data_fields
        );
    } else {
        eprintln!(
            " {} Unspecified - {} Bytes, {} Data",
            count, format.blob_size, format.data_fields
        );
    }

    // Decode contents.
    for c in 0..count {
        // Decode blob.
        if format.blob_size > 0 {
            if format.blob_size != 1 || format.data_fields > 0 || c == 0 {
                eprint!("|.");
                for _ in 0..depth {
                    eprint!(".");
                }
                eprint!(" ");
            }

            for _ in 0..format.blob_size {
                let mut buffer = [0u8; 1];
                assert_eq!(1, data.read(&mut buffer)?);
                eprint!("{}", hex_from_bytes(&buffer));
            }

            if format.blob_size != 1 || format.data_fields > 0 || c + 1 == count {
                eprintln!();
            } else {
                eprint!("..");
            }
        }

        // Decode data.
        for _ in 0..format.data_fields {
            inspect_data(data, depth + 1)?;
        }
    }

    Ok(())
}
