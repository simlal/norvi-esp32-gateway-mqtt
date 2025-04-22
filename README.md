# ESP32 as Gateway to MQTT broker/SQL database

Make a ESP32 Gateway connect to MQTT an broker with message persistence and a SQL database to store the data.

## Introduction

### Overview

This Project is a simple end-to-end IoT Project by leveraging a NORVII microcontroller (ESP32-WROOM + OLED Display)
to connect to the local WiFi as a gateway to a MQTT Broker.
This gateway device uses its wifi connection to connect to the MQTT broker where it will publish the data to a topic based on its MAC address.
The data is then processed by a multitasking Flask application that both interacts with the MQTT broker (in both PubSub mode) and persists the data to a SQL database. The code to the Front-end/Backend/Database app can found in this [code repository](https://github.com/simlal/mqttbroker-sqlite-flask_minimalist-stack)

A simple demonstration of the retry connection and full MQTT-Flask-Sqlite stack in action can be found [here](TODO) with the [matching display](TODO)

### Rust and embassy

This project is written in Rust using the [embassy](https://docs.rs/embassy/latest/embassy/) framework. The embassy framework is a no-std async framework for embedded systems that allows you to write asynchronous code in Rust.
It is designed as a replacement for Real-Time Operating Systems (RTOS) and is based on the async/await syntax of Rust. The embassy framework is designed to be lightweight and compatible with many SDKs (including `esp-hal` from the chip manufacturer of ESP32), making it ideal for embedded systems with limited resources
while retaining the performance and memory safety features of rust.

Leveraging the connectivity capabilities of the ESP32, the project can be extended to include a mesh network of ESP32 devices that can send data to the gateway device using ESP-NOW protocol. The gateway device will then publish the data to the MQTT broker.

### ⚠️ IMPORTANT (BUG FIX/HACK) ⚠️

Sensor integration for the gateway was only started and not working because of the need to share the I2C bus between the OLED display and the sensors. The I2C bus is a shared bus, meaning that only one device can be connected to it at a time.

Thus we need to actually share a mutex to the I2C modules between the display task and the create a new temperature task instead of in the main loop.

In the meantime, a hack was added to generate random temperature based on system clock and still displaying and publishing the temperature to the broker with a fake MAC address.

## Hardware

This project uses an ESP32-WROOM-based microcontroller with integrated OLED display (NORVII) as the central gateway, powered by a 12V DC supply through a custom PCB for power management and signal processing.

The ESP32 microcontroller is an ideal choice for our IoT system due to its
processing capabilities (2 cores, 32-bit registers, 160 MHz, 520KB SRAM/4MB Flash)
and built-in WiFi and Bluetooth connectivity. With its integrated analog-to-digital
converter, it can read current signals from sensors and transmit them to a cloud
server via WiFi.

Theoretically, if WiFi connectivity is lost, the microcontroller can temporarily store data locally
in its 4MB flash memory and send it once the connection is restored. It can also
encrypt data before sending it to the cloud server for enhanced security by using TLS.

**Hardware requirements:**

- ESP32-WROOM microcontroller with OLED display (NORVII)
- Power supply (12V DC)

### Bill of Materials

- ESP32-WROOM microcontroller with integrated OLED display (NORVII)
- 12V DC power supply
- Custom PCB for power distribution and signal conversion
- _(OPTIONAL)_: additional ESP32-based sensor nodes for mesh network

## Installation and configuration

This project is Rust-based and uses the embassy framework, thus it requires a Rust toolchain to be installed on your system. The project is designed to be built and run on the ESP32 microcontroller, so you will need to install the ESP32 toolchain as well.

### Pre-requisites

- Rust and Cargo installed
- `espup` and `cargo-espflash` (or `espflash`) tools installed
- ESP-IDF (ESP32 development framework)
- WiFi network with internet access

Also, for actual data collection, you will need to have:

- MQTT broker running (see companion repository for setup)

**NOTE**: To activate the LSP (Language Server Protocol) for Rust in your IDE, you need to install the `rust-analyzer` extension. This will provide you with features like code completion, error checking, and more.
Since this is on the esp toolchain, we need to symlink the `rust-analyzer` to the `cargo` directory of the esp toolchain. This is done by running the following command:

```bash
# With the stable toolchain already installed
cd ~/.rustup/toolchains/esp/bin
ln -s ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rust-analyzer rust-analyzer
```

Make sure we have everything:

```bash
cargo install espup
cargo install espflash # or brew install espflash

# Clone the repo and source the ENV var for the ESP linker
git clone git@github.com:simlal/norvi-esp32-gateway-mqtt.git
source export-esp.sh
```

### Configuration

Since this is a toy project, Wifi SSID has to be put in the `.cargo/config.toml` file in the root of the project. The password is set in the environment when compiling and flashing the code.

```toml
[env]
ESP_LOG = "DEBUG"
SSID = "YOUR_WIFI_SSID"
```

We also set the log level to DEBUG to see the logs in the console.

## IoT Architecture and software

We are using a simple architecture with a single ESP32 microcontroller as the gateway device. The ESP32 microcontroller is connected to the local WiFi network and acts as a gateway to the MQTT broker. The ESP32 microcontroller is also connected to an OLED display that shows the status of the device and the data being sent to the MQTT broker since the last refresh.

By separating modules into different files, we can easily add new features and functionalities to the project. The main modules are:

- `gateway_lib`: Contains the `display.rs` and `requests.rs` modules that handle the display and http requests for Internet API calls.
- `common`: Contains the `wifi.rs` module that handles the wifi connection.

### ESP32 Gateway

To compile the code for the main gateway device, you need to run the following command:, we plug the ESP32 gateway via microUSB and run:

```bash
# Specifing dialout group to make sure we have access to the serial port
SSID_PASSWORD='YourVerySecurePassword' sg dialout -c "cargo run --bin main_gateway --release"
```

It will then spawn a 2 tasks:

1. Display/data refresh task
2. Wifi Connection task

And in the main loop: Try to connect to the MQTT broker and publish data to the topic based on the MAC address of the device.

Here the data is simply the RSSI value of the wifi connection, but it can be extended to include other data from sensors connected to the ESP32 microcontroller.

#### Display task

The display task manages the OLED interface, providing real-time system status information.
It runs as a continuous async task that:

- Updates and displays the elapsed time since last refresh
- Shows WiFi signal strength as a percentage based on RSSI values
- Displays MQTT connection status (Offline, Connected, Disconnected, Published, Error)
- Refreshes the display every 5 seconds

The task uses atomic variables to safely share status information between threads, and
the embedded-graphics library to render text on the SSD1306 OLED display. This provides
users with immediate visual feedback about system operation and connectivity.

#### Wifi Connection task

The WiFi connection task manages network connectivity, with these key functions:

- Establishes and maintains the WiFi connection using provided credentials
- Automatically reconnects when connectivity is lost
- Regularly scans for access points to monitor signal strength (RSSI)
- Stores current signal strength in an atomic variable for thread-safe access
- Converts raw RSSI values (-90 to -30 dBm) to user-friendly percentages (0-100%)
- Waits for IP address assignment before proceeding with other network operations

The connection task implements a resilient approach with labeled loops that handle
different states (connection establishment, monitoring) and gracefully recovers from
disconnections with appropriate retry intervals.

#### Main Loop

The main loop handles MQTT connectivity and data publishing:

1. **Initialization**:
   - Sets up hardware, allocates heap memory, and initializes the Embassy framework
   - Configures I2C for the OLED display and spawns the display update task
   - Initializes WiFi in STA (station) mode and connects to the configured network
   - Sets up the network stack with DHCP for IP assignment

2. **Connection Management**:
   - Attempts to connect to the MQTT broker every 30 seconds
   - Uses the device's MAC address as the client ID for unique identification
   - Implements robust error handling with appropriate status codes for display
   - Sets connection timeouts to prevent hanging on failed connections

3. **Data Publishing**:
   - Creates a unique topic based on the device's MAC address for publishing data
   - Collects WiFi signal strength (RSSI) data and converts it to percentage
   - Formats data as JSON including MAC address, timestamp, and signal strength
   - Publishes with QoS1 to ensure delivery acknowledgment
   - Updates status indicators visible on the OLED display

The main loop implements a resilient design that handles connectivity issues by
continuously attempting to reconnect, while providing visual feedback on the system via the display and logs.

### Sensor Mesh

**BONUS IF I HAVE TIME**: Use the additionnal custom PCBs (ESP32-WROOM based) to create a mesh network and use ESP-NOW protocol to send data to the gateway device. The ESP32-WROOM based devices will be used as sensors and will send data to the gateway device using ESP-NOW protocol. The gateway device will then publish the data to the MQTT broker.

## Further work

We could easily add a temperature/humidity sensor to the ESP32 microcontroller and publish the data to the MQTT broker, which would be more meaningful than the RSSI value of the wifi connection.

Also, implementing the mesh sensor network using ESP-NOW protocol would be a great addition to the project. This would allow us to have multiple sensor nodes that can send data to the gateway device without the need for a WiFi connection. The gateway device would then publish the data to the MQTT broker.

## Conclusion

This project demonstrates a complete end-to-end IoT solution using ESP32 as a gateway
to connect sensors to an MQTT broker with SQL database persistence. By leveraging Rust
and the Embassy framework, we've created a reliable, memory-safe system with:

- Real-time data collection and transmission
- OLED-based visual status feedback
- Resilient connectivity with automatic reconnection
- Thread-safe data sharing between components
- Structured JSON data publishing to MQTT

The modular design allows for easy extension with additional sensors or mesh networking
capabilities. This gateway serves as a solid foundation for more complex IoT applications
that require reliable connectivity, data persistence, and visualization.

## References

- [ESP32-WROOM datasheet](https://www.espressif.com/sites/default/files/documentation/esp32-wroom-32_datasheet_en.pdf)
- [Embassy framework documentation](https://docs.rs/embassy/latest/embassy/)
- [Rust-MQTT client library](https://docs.rs/rust-mqtt/latest/rust_mqtt/)
- [Embedded-graphics library](https://docs.rs/embedded-graphics/latest/embedded_graphics/)
- [SSD1306 OLED display driver](https://docs.rs/ssd1306/latest/ssd1306/)
- [MQTT protocol specification](https://mqtt.org/mqtt-specification/)
