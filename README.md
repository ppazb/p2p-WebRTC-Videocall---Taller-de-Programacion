# Taller de Programacion - Grupo Sobrevivir Taller

En este presente trabajo desarrollamos una versión del Stack WebRTC en el lenguaje Rust, de manera tal que sea posible la realización de videoconferencia entre dos usuarios en distintos dispositivos. 

Los componentes para esta instancia son: 

- SDP (Session Description Protocol)
- ICE (Interactive Connectivity Establishment)
- Transmisión de Video (Encodeo y Desencodeo)
- Interfaz UI de la Aplicación

## Integrantes

- Rivero, Tobías 106302
- Vargas, Alejandro 104121
- Paz Blanco, Pilar 105600
- Chen, Michelle Melisa 105506


## Como usar 

A continuacion se detallan los pasos para compilar y ejecutar el programa. 

En la terminal de cada dispositivo: 

```
cargo run --bin room_rtc  <IP:Puerto local> <IP:Puerto peer>

Dispotivo 1: 
cargo run --bin room_rtc 127.0.0.1:4000 127.0.0.2:5000

Dispositivo 2: 
cargo run --bin room_rtc 127.0.0.2:5000 127.0.0.1:4000
```
En caso de probar en un mismo dispositivo abriendo dos terminales: 

```
cargo run --bin room_rtc  <IP:Puerto local> <IP:Otro Puerto local>

Terminal 1: 
cargo run --bin room_rtc 127.0.0.1:4000 127.0.0.1:5000

Terminal 2: 
cargo run --bin room_rtc 127.0.0.1:5000 127.0.0.1:4000
```