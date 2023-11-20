use http::Response;

pub trait ParseBytes {
    fn parse_to_bytes(self) -> Vec<u8>;
}

impl ParseBytes for Response<String> {
    fn parse_to_bytes(self) -> Vec<u8> {
        let (headers, body) = self.into_parts(); 
        let mut response : Vec<u8> = Vec::new(); 
        response.extend_from_slice("HTTP/1.1 ".as_bytes()); // Always using this protocol
        response.extend_from_slice(headers.status.as_str().as_bytes());
        response.push(b' ');
        response.extend_from_slice(headers.status.canonical_reason().unwrap().as_bytes());
        response.extend_from_slice("\r\n".as_bytes());
        for (key, val) in headers.headers {
            response.extend_from_slice(key.unwrap().as_str().as_bytes());
            response.extend_from_slice(": ".as_bytes());
            response.extend_from_slice(val.as_bytes());
            response.extend_from_slice("\r\n".as_bytes());
        }
        response.extend_from_slice("\r\n".as_bytes());
        response.extend_from_slice(body.as_bytes());
        response
    }
}
