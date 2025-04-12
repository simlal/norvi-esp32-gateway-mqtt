# REPORTLOG

Progress report for IFT-744 IoT Project

## Progress report for 06/03/2025

- Finished initial setup with `cargo` and esp-rs toolchain
- Hello world info log

## Progress report for 13/03/2025

- Finished the initial setup of the project
- Made display and refresh work for Last Updated, Wifi Level and MQTT Status

## Progress for 20/03/2025

- Fixed issues with the display update time
- Started working on the Wifi implementation for the ESP32 Gateway
- Able to read the env vars for SSID and password
- Used tasks to poll the connection to wifi
- Made RSSI polling with a bit of a hack within the connection task
- Updated the RSSI to the device display

## Progress for 27/03/2025

**BACKEND-SIDE:**

- Created repo for MQTT-SQL-Flask stack: <https://github.com/simlal/MqttBroker-SQLite-Flask_minimalist-stack>
- Base image file for MQTT-SQL-Flask stack with `docker compose`
- Made the SQLite init tables with minimalist schema
- Created endpoints on Flask to Consume the data from the SQL db and publish to MQTT

**FIRMWARE:**

- Made wifi connection work with the ESP32 gateway
- Moved the display update to a non blocking task
- Tested a GET request from the Gateway to the Flask app.
