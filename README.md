# ESP32 as Gateway to MQTT broker/SQL database

Make a ESP32 Gateway connect to MQTT an broker with message persistence and a SQL database to store the data.

## Introduction

This Project is a simple end-to-end IoT Project
This project uses a NORVII (ESP32-WROOM + OLED Display) to connect to the local WiFi as a gateway to a MQTT Broker.
This gateway device uses its wifi connection to connecto to the MQTT broker where it will publish the data to a topic.
This data is then processed by a multitasking Flask application that both interacts with the MQTT broker (in both PubSub mode) and persists the data to a SQL database.

**BONUS IF TIME IS LEFT**: Use the additionnal custom PCBs (ESP32-WROOM based) to create a mesh network and use ESP-NOW protocol to send data to the gateway device. The ESP32-WROOM based devices will be used as sensors and will send data to the gateway device using ESP-NOW protocol. The gateway device will then publish the data to the MQTT broker.

This repo only contains the code for the ESP32 devices (mesh sensors) and ESP32 Gateway. For the MQTT broker, SQL database and web app, please refer to the following [repo](https://github.com/simlal/MqttBroker-SQLite-Flask_minimalist-stack)

## Hardware

TODO

### Bill of Materials

TODO

## Installation and configuration

### Pre-requisites

TODO

### Configuration

TODO

## IoT Architecture and software

TODO

### ESP32 Gateway

TODO

### Sensor Mesh

TODO

### Backend (MQTT-SQL database)

TODO

## Data analysis
