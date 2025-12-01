# Taller de Programacion - Grupo Sobrevivir Taller

En este presente trabajo desarrollamos una versión del Stack WebRTC en el lenguaje Rust, de manera tal que sea posible la realización de videoconferencia entre pares de usuarios. 

Los componentes son: 

- SDP (Session Description Protocol)
- ICE (Interactive Connectivity Establishment)
- Servidor STUN
- Comunicación encriptada con el Signalling Server (TLS)
- Transmisión de Video (Encodeo y Desencodeo)
- SRTP (Secure Real-time Transport Protocol)
- DTLS (Datagram Transport Layer Security)
- Interfaz UI de la Aplicación

## Integrantes

- Rivero, Tobías 106302
- Vargas, Alejandro 104121
- Paz Blanco, Pilar 105600
- Chen, Michelle Melisa 105506


## Como usar 

A continuacion se detallan los pasos para compilar y ejecutar el programa. 

Para levantar el Signalling Server: 

```
cargo run --bin signalling_server  path_config_file
```

Para levantar el Client en una terminal: 

```
cargo run --bin room_rtc  path_config_file
```

### Ejemplo de configs para server y client

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


