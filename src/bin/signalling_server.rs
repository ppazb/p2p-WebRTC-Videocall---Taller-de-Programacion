use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::net::TcpStream;
use std::sync::{Arc, mpsc};

use room_rtc::server_users::{self, ServerUserData};
use room_rtc::config::*;
use room_rtc::signalling_error::SignallingServerError;
use serde::Deserialize;

fn main() -> Result<(), SignallingServerError> {
    // cargo run --bin signalling_server src/server_config.rs
    // Para obtener el config de Signalling
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err(SignallingServerError::ConnectionError);
    }

    let server_info: SignallingServerParameters = match get_config_signalling(&args[1]) {
        Ok(info) => info,
        Err(e) => {
            eprintln!("\n[DEBUG] SERVER - Config Info Error: {:?}", e);
            return Err(SignallingServerError::ConfigInfoError);
        }
    };
    println!("\n\n[DEBUG] SERVER - Config Info: {:?}", server_info);

    let file_path = &server_info.users_db;
    let login = Arc::new(server_users::ServerUserData::new(file_path).unwrap());
    
    let listener = std::net::TcpListener::bind(&server_info.server_adder)
        .map_err(|_| SignallingServerError::ConnectionError)?;

    //let (tx, rx) = mpsc::channel::<User>();
    let (tx, rx) = mpsc::channel::<Result<(), SignallingServerError>>();
    
    

    let mut handles = Vec::new();
    let mut clients = HashMap::<String, User>::new();

    let listener_thread_handle = std::thread::spawn(move || {
        
        let mut client_handles = Vec::new();
        
        for stream in listener.incoming() {
            println!("[DEGUE] Llegué");
            let login_signalling = Arc::clone(&login); 
            match stream {
                Ok(s) => {
                    println!("\n\n[DEBUG] SERVER - Connection User Succeded");
                    let tx_user_info = tx.clone();
                    let new_client = std::thread::spawn(move || {
                        login_handle(s, tx_user_info,login_signalling );
                    }
                        //client_handle(s, tx_user_info));
                    );
                    client_handles.push(new_client);
                }

                Err(e) => {
                    eprintln!("\n[DEBUG] SERVER - Connection User Error")
                }
            }
        }
    });

    handles.push(listener_thread_handle);
    // for user in rx.iter() {
    //     println!("\n\n[DEBUG] SERVER - User Info: {:?}", user);
    //     clients.insert(user.user_info.user_id.to_string(), user);
    //     println!("\n\n[DEBUG] SERVER - HashMap Users: {:?}", clients);
    // }

    Ok(())
}


fn login_handle(mut stream: TcpStream, tx: mpsc::Sender<Result<(), SignallingServerError>>, loggin: Arc<ServerUserData>  ) {
    let mut reader = BufReader::new(&mut stream);
    let mut signal_str = String::new();
    let _ = reader.read_to_string(&mut signal_str);
    println!("\n\n[DEBUG] SERVER - Login Information {:?}", signal_str); 
    
}

fn client_handle(mut stream: TcpStream, tx: mpsc::Sender<User>) {
    let mut reader = BufReader::new(&mut stream);
    let mut signal_str = String::new();
    let _ = reader.read_to_string(&mut signal_str);
    println!("[DEBUG] SERVER - Client Handle: {:?}", signal_str);

    let user_info: UserInfo = match serde_json::from_str(&signal_str) {
        Ok(user) => user,
        Err(e) => {
            eprintln!("\n[DEBUG] SERVER - Deserialize User Infor: {:?}", e);
            return;
        }
    };

    let user: User = User {
        user_info,
        socket: stream,
    };

    tx.send(user);
}

#[derive(Debug, Deserialize, PartialEq, Default)]
pub struct UserInfo {
    pub user_name: String,
    pub user_id: String,
    pub password: String,
    pub offer: String,
}

#[derive(Debug)]
pub struct User {
    pub user_info: UserInfo,
    pub socket: TcpStream,
}
