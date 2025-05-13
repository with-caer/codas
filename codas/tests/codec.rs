#![cfg(test)]
//! Integration tests of the codec over `std` IO sockets.

use std::{
    net::{TcpListener, TcpStream},
    thread::JoinHandle,
};

use codas::{
    codec::{CodecError, ReadsDecodable, WritesEncodable},
    types::Text,
};

#[test]
pub fn test_codec_over_tcp() -> Result<(), CodecError> {
    // Create our request data.
    let request_data = TestMessage {
        number: 9000,
        text_list: vec!["I like cake.".into()],
        text: "Hello, Codecs!".into(),
    };

    // We'll add this string to the response,
    // and increment the response number by 1.
    let additional_string = Text::from("The cake is a lie.");

    // Create our _expected_ response data.
    let expected_response_data = TestMessage {
        number: 9001,
        text_list: vec!["I like cake.".into(), additional_string.clone()],
        text: "Hello, Codecs!".into(),
    };

    // Create TCP listener on an arbitrary port, configuring
    // it to echo our request with some additional data.
    let listener = TcpListener::bind("0.0.0.0:0").unwrap();
    let listener_port = listener.local_addr().unwrap().port();
    let expected_request_data = request_data.clone();
    let server: JoinHandle<Result<(), CodecError>> = std::thread::spawn(move || {
        // Accept the first client request and decode it.
        let (mut socket, _) = listener.accept().unwrap();
        let mut request_data = socket.read_data()?;
        assert_eq!(expected_request_data, request_data);

        // Append additional data to the test data.
        request_data.number += 1;
        request_data.text_list.push(additional_string);

        // Send it back to the client.
        socket.write_data(&request_data)?;

        Ok(())
    });

    // Create a TCP client connection to the listener
    // and send some encoded data.
    let mut client = TcpStream::connect(format!("0.0.0.0:{listener_port}")).unwrap();
    client.write_data(&request_data)?;

    // Decode the response.
    let response_data = client.read_data()?;
    assert_eq!(expected_response_data, response_data);

    // Join server to ensure no errors occurred on it's side.
    server.join().unwrap()?;

    Ok(())
}

// Auto-generated via: export_coda!("codas/tests/test_coda.md");
const _: &str = "";
#[doc = "Undocumented Coda. How could you? ;~;"]
#[derive(Clone, Debug, PartialEq)]
pub enum TestData {
    #[doc = "Unspecified data."]
    Unspecified(codas::types::Unspecified),
    #[doc = "Undocumented Type. How could you? ;~;"]
    TestMessage(self::TestMessage),
}
impl TestData {
    #[doc = " This variant\'s ordinal in the coda."]
    pub fn ordinal(&self) -> codas::codec::FormatMetadata {
        match self {
            Self::Unspecified(..) => 0,
            Self::TestMessage(..) => 1,
        }
    }
}
impl codas::codec::Encodable for TestData {
    const FORMAT: codas::codec::Format = codas::codec::Format::Fluid;
    fn encode(
        &self,
        writer: &mut (impl codas::codec::WritesEncodable + ?Sized),
    ) -> core::result::Result<(), codas::codec::CodecError> {
        match self {
            Self::Unspecified(data) => data.encode(writer),
            Self::TestMessage(data) => data.encode(writer),
        }
    }

    fn encode_header(
        &self,
        writer: &mut (impl WritesEncodable + ?Sized),
    ) -> Result<(), CodecError> {
        match self {
            Self::Unspecified(data) => data.encode_header(writer),
            Self::TestMessage(data) => data.encode_header(writer),
        }
    }
}
impl codas::codec::Decodable for TestData {
    fn decode(
        &mut self,
        reader: &mut (impl codas::codec::ReadsDecodable + ?Sized),
        header: Option<codas::codec::DataHeader>,
    ) -> core::result::Result<(), codas::codec::CodecError> {
        let header = Self::ensure_header(header, &[0, 1])?;
        match header.format.ordinal {
            0 => {
                let mut data = codas::types::Unspecified::default();
                data.decode(reader, Some(header))?;
                *self = Self::Unspecified(data);
                Ok(())
            }
            1 => {
                let mut data = self::TestMessage::default();
                data.decode(reader, Some(header))?;
                *self = Self::TestMessage(data);
                Ok(())
            }
            _ => panic!("internal error: entered unreachable code"),
        }
    }
}
impl core::default::Default for TestData {
    fn default() -> TestData {
        Self::Unspecified(codas::types::Unspecified::default())
    }
}
#[doc = "Undocumented Type. How could you? ;~;"]
#[derive(Default, Clone, Debug, PartialEq)]
pub struct TestMessage {
    pub number: i32,
    pub text_list: Vec<codas::types::Text>,
    pub text: codas::types::Text,
}
impl codas::codec::Encodable for TestMessage {
    const FORMAT: codas::codec::Format = codas::codec::Format::data(1)
        .with(<i32 as codas::codec::Encodable>::FORMAT)
        .with(<Vec<codas::types::Text> as codas::codec::Encodable>::FORMAT)
        .with(<codas::types::Text as codas::codec::Encodable>::FORMAT);
    fn encode(
        &self,
        writer: &mut (impl codas::codec::WritesEncodable + ?Sized),
    ) -> core::result::Result<(), codas::codec::CodecError> {
        writer.write_data(&self.number)?;
        writer.write_data(&self.text_list)?;
        writer.write_data(&self.text)?;
        Ok(())
    }
}
impl codas::codec::Decodable for TestMessage {
    fn decode(
        &mut self,
        reader: &mut (impl codas::codec::ReadsDecodable + ?Sized),
        header: Option<codas::codec::DataHeader>,
    ) -> core::result::Result<(), codas::codec::CodecError> {
        let _ = Self::ensure_header(header, &[1])?;
        reader.read_data_into(&mut self.number)?;
        reader.read_data_into(&mut self.text_list)?;
        reader.read_data_into(&mut self.text)?;
        Ok(())
    }
}
impl From<codas::types::Unspecified> for TestData {
    fn from(data: codas::types::Unspecified) -> TestData {
        TestData::Unspecified(data)
    }
}
impl From<self::TestMessage> for TestData {
    fn from(data: self::TestMessage) -> TestData {
        TestData::TestMessage(data)
    }
}
