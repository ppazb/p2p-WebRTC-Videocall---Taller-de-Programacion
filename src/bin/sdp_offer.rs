use room_rtc::sdp_protocol::*; 


fn main() {

    // cargo run --bin sdp_offer <ip:port> <file_path_offer>

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        return;
    }

    let _ = create_sdp_offer(&args[1], &args[2]); 

}

