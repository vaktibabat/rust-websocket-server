use base64::{engine::general_purpose, Engine as _};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::io::{prelude::*};
use std::net::{Shutdown, TcpListener, TcpStream};

// CRLF Sequence
const CRLF: &str = "\r\n";

fn handle_client(mut stream: TcpStream) {
    let mut data = [0 as u8; 1024];

    // Stream can change (for example client can write multiple times) so this is done in a while loop
    while match stream.read(&mut data) {
        Ok(_) => {
            // Is this a GET request (HTTP handshake)
            // 0x47 = 'G'
            // 0x45 = 'E'
            // 0x54 = 'T'
            if data[0] == 0x47 && data[1] == 0x45 && data[2] == 0x54 {
                // Currently the data is an array that contains the ascii values, so now that we know that it is an HTTP request
                // convert it into a string
                let data = String::from_utf8(data.to_vec()).unwrap();
                // Get the individual headers
                let mut headers: Vec<_> = data.lines().take_while(|x| !x.is_empty()).collect();
                // Remove the first line (GET /path)
                headers.remove(0);
                // Put in a HashMap for a small performance boost and to be more clear
                let mut headers_hashmap: HashMap<&str, &str> = HashMap::new();

                // Go over headers and put in the HashMap
                for header in headers {
                    let split_header: Vec<_> = header.trim().split(": ").collect();
                    headers_hashmap.insert(split_header[0], split_header[1]);
                }

                // Doing some error checking on the headers
                if headers_hashmap.get("Upgrade").unwrap() != &"websocket" 
                || !headers_hashmap.get("Connection").unwrap().contains("Upgrade")
                || headers_hashmap.get("Sec-WebSocket-Version").unwrap() != &"13" {
                    println!("Invalid Headers");
                    let _ = stream.shutdown(Shutdown::Both);
                    return;
                }

                let mut hasher = Sha1::new();

                // Constructing the new key for Sec-WebSocket-Accept
                hasher.update(
                    (headers_hashmap.get("Sec-WebSocket-Key").unwrap().to_owned().to_owned() + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").as_bytes()
                );

                let hasher_res = hasher.finalize();

                // Constructing the response
                let response = "HTTP/1.1 101 Switching Protocols".to_owned()
                    + CRLF
                    + "Connection: Upgrade"
                    + CRLF
                    + "Upgrade: websocket"
                    + CRLF
                    + "Sec-WebSocket-Accept: "
                    + &general_purpose::STANDARD.encode(hasher_res)
                    + CRLF
                    + CRLF;

                // And sending it
                stream.write_all(response.as_bytes()).unwrap();
            }
            else {
                // Message is a WebSocket. We're not doing error checking here to not make the code more complex
                let payload_length = data[1] & 0b01111111;
                let mut decoded_payload: Vec<u8> = vec![];

                if payload_length < 126 {
                    let mask = &data[2..6];

                    for i in 0..payload_length {
                        decoded_payload.push(data[6 + i as usize] ^ mask[i as usize % 4]);
                    }
                }
                else if payload_length == 126 {
                    // Next two bytes are the real payload length
                    let payload_length = ((data[2] as u32) << 8) + data[3] as u32;
                    let mask = &data[4..8];
                    
                    for i in 0..payload_length {
                        decoded_payload.push(data[8 + i as usize] ^ mask[i as usize % 4]);
                    }
                }
                else if payload_length == 127 {
                    // Next eight bytes are the real payload length
                    let payload_length = ((data[2] as u64) << 56) +
                    ((data[3] as u64) << 48) +
                    ((data[4] as u64) << 40) +
                    ((data[5] as u64) << 32) +
                    ((data[6] as u64) << 24) +
                    ((data[7] as u64) << 16) +
                    ((data[8] as u64) << 8) +
                    data[9] as u64;
                    let mask = &data[10..14];

                    for i in 0..payload_length {
                        decoded_payload.push(data[14 + i as usize] ^ mask[i as usize % 4]);
                    }
                }

                let decoded_payload = String::from_utf8(decoded_payload).unwrap();

                println!("Received the message {}", decoded_payload);

                if decoded_payload == "hello" {
                    stream.write_all(&data).unwrap();
                }
            }
            
            true
        },
        Err(e) => {
            println!("err: {:?}", e);
            false
        }
    } {}
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8000").unwrap();
    println!("Listening on 127.0.0.1:8000");
    
    // For every incoming connection
    for stream in listener.incoming() {
        let stream = stream.unwrap();
    
        println!("Connection established with {}", stream.peer_addr().unwrap());
        // We'll write this function later
        handle_client(stream);
    }
    
    // Stop listening
    drop(listener)
}
