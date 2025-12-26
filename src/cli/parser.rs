use std::net::SocketAddr;

pub fn parse_string(input: String) -> Result<(SocketAddr, Vec<String>), Box<dyn std::error::Error>> {
    let args = input.split_whitespace().collect::<Vec<&str>>();
    let addr = args.last().unwrap().to_string();

    let socket_addr: SocketAddr = addr.parse()?;

    let string_vec: Vec<String> = args.iter().map(|&s| s.to_string()).collect();
    Ok((socket_addr, string_vec))
}