# Taller de Programacion I - Grupo Sobrevivir Taller

En este presente proyecto desarrollamos una aplicación de videoconferencia Peer-to-Peer (P2P) basada en la arquitectura del stack WebRTC en Rust. La aplicación transmisión de audio/video y transferencia de archivos en tiempo real permitiendo, también, la creación de nuevos usuarios con sus contraseñas para inicio de sesión, apagado/encendido del micrófono y cámara, 

El trabajo principal fue implementar la pila de protocolos del stack WebRTC algunos desde cero y otros con la ayuda de crates. 

Los componentes son: 

- **SDP (Session Description Protocol)**: utilizado para establecer la conexión entre pares. 
- **ICE (Interactive Connectivity Establishment)**: Protocolo de comunicación utiliazado para definir la ruta por la que se enviará el video.
- **Servidor STUN**: Soporte de direcciones Server Reflexive (STUN). Para poder utilizar servidor público.
- **Comunicación encriptada con el Signalling Server (TLS)**: Mensajes entre el usuario y el servidor encriptados.
- **Transmisión de Video (Encodeo y Desencodeo)**: Códec H264.
- **SRTP (Secure Real-time Transport Protocol)**
- **DTLS (Datagram Transport Layer Security)**
- **Archivo de Configuración**: Los componentes de la aplicación pueden ser configurados mediante archivos de configuración. Elegirmos usar .json. 
- **Inicio de Sesión, creación de usuarios nuevos y visualización de los estados de conexión**
- **Interfaz UI de la Aplicación**: Implementación de vistas como Inicio de Sesión, Sala de Espera y Sala de Llamada. Visualización de estadíasticas de transmisión.
- **Logs**: Mantenemos registros de las acciones realizadas en Client y Server.

## UI

### Inicio de Sesión 
<img width="1847" height="1023" alt="image" src="https://github.com/user-attachments/assets/bb8acaa7-9682-4a7d-8369-89565de29fbd" />

### Creación de nuevos usuarios

<img width="1854" height="1048" alt="image" src="https://github.com/user-attachments/assets/293522ad-91ca-4c6c-be6c-3d3507cc17b6" />

### Inicio de Sesión con usuario y contraseña
<img width="1854" height="1048" alt="image" src="https://github.com/user-attachments/assets/6a775393-b133-4171-81a1-77437f898d3f" />

### Sala de Espera
<img width="1854" height="1048" alt="image" src="https://github.com/user-attachments/assets/f68e851c-f360-4224-977e-ee5980619d00" />

### Visualización de la lista de usuarios conectados 

<img width="1854" height="1048" alt="image" src="https://github.com/user-attachments/assets/c62c5839-c536-49bf-8642-d6b7058e438a" />

### Llamada

<img width="1854" height="1048" alt="image" src="https://github.com/user-attachments/assets/21ed253e-866c-4fe1-8292-152bb7d8b871" />
<img width="1854" height="1048" alt="image" src="https://github.com/user-attachments/assets/5a8e3a32-ede3-49eb-be88-79e2fc7719de" />

### En Llamada 

<img width="1854" height="1048" alt="image" src="https://github.com/user-attachments/assets/203ea7f6-a216-46bc-a48e-7a7be94c4f28" />

## Integrantes

- Rivero, Tobías 106302
- Vargas, Alejandro 104121
- Paz Blanco, Pilar 105600
- Chen, Michelle Melisa 105506


## Cómo usar 

A continuación se detallan los pasos para compilar y ejecutar el programa. 

Para levantar el Signalling Server: 

```
cargo run --bin signalling_server  path_config_file
```

Para levantar el Client en una terminal:  

```
cargo run --bin room_rtc  path_config_file
```

### Ejemplo de archivo de configuración para server y client

```
{
    "server_adder": "localhost:8000",
    "users_db": "src/users.json"
}
```



```
{
    "general_info": {
        "server_adder": "192.168.0.15"
    },

    "sdp_handshake": {
        "local_adder": "localhost:8090",
        "server_adder": "192.168.0.15:8000", 
        "codecs": ["VP8", "H264"],
        "ice_candidates": [ "192.168.0.114:8500", "192.168.0.114:8250"],
        "offer_path": "src/offer2.sdp"
    }
}


```


