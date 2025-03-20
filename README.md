# espnow-mesh-temp-monitoring-rs

Make a simple mesh network of temperature probes using ESP-NOW and a ESP32 Gateway to MQTT broker/SQL database for an advanced IoT course at UdeS (IFT-744)

## Introduction

This project is to create a mesh network of ESP32 devices that will send temperature data to a gateway device.
The gateway device will then connect to a MQTT broker where it will publish the data to a topic.
The data will be stored in a database and will be available to be queried by a simple web app.

Leveraging the power of Rust, Embassy (Async runtime for embedded systems) and esp-rs crates, we will create a simple program that will allow us to create a mesh network of ESP32 devices that will send temperature data to a gateway device. This gateway device will then connect to a MQTT broker where it will publish the data to a topic. The data will be stored in a database and will be available to be queried by a simple web app.

This repo only contains the code for the ESP32 devices (mesh sensors) and ESP32 Gateway. For the MQTT broker, SQL database and web app, please refer to the following [repo]()

## Hardware

## Bill of Materials

## Architecture

##
