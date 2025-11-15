use std::collections::HashMap;
use std::io::{BufReader, Read, Write};
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
    let (tx, rx) = mpsc::channel::<Result<TcpStream, SignallingServerError>>();
    
    // let mut handles = Vec::new();
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
            }; 
        }

        for handle in client_handles {
            handle.join().unwrap(); 
        }; 
    });

    for answer in rx.iter() {
        match answer { 
            Ok(mut stream) => {
                let _ = stream.write_all(b"OK\n");
            },
            Err(e) => {
                eprintln!("[ERROR] Server error: {:?}", e);
                return Err(e);
            }
        };
    }


    listener_thread_handle.join(); 
    // handles.push(listener_thread_handle);
    // for user in rx.iter() {
    //     println!("\n\n[DEBUG] SERVER - User Info: {:?}", user);
    //     clients.insert(user.user_info.user_id.to_string(), user);
    //     println!("\n\n[DEBUG] SERVER - HashMap Users: {:?}", clients);
    // }

    Ok(())
}


fn login_handle(mut stream: TcpStream, tx: mpsc::Sender<Result<TcpStream, SignallingServerError>>, loggin: Arc<ServerUserData>) {
    use std::io::BufRead;
    
    let mut reader = BufReader::new(&stream);
    let mut username = String::new();
    let mut password = String::new();
    
    // Leer username (primera línea)
    if let Ok(n) = reader.read_line(&mut username) {
        if n > 0 {
            let user_trimmed = username.trim().to_string();
            eprintln!("\n[DEBUG SERVER] Received username: {}", user_trimmed);
        }
    }
    
    // Leer password (segunda línea)
    if let Ok(n) = reader.read_line(&mut password) {
        if n > 0 {
            let pass_trimmed = password.trim().to_string();
            eprintln!("[DEBUG SERVER] Received password: {}", pass_trimmed);
        }
    }
    
    eprintln!("[DEBUG SERVER] Sending OK response...");
    let mut stream_write = &stream;
    let _ = stream_write.write_all(b"OK\n");
    let _ = stream_write.flush();
    eprintln!("[DEBUG SERVER] Response sent!\n");
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
