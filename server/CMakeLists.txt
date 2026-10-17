cmake_minimum_required(VERSION 4.3)
project(server)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

add_executable(server
        main.cpp
        include/socket.h
        source/Logger.cpp
        include/Logger.h
        include/network/serialization/PacketReader.h
        source/network/serialization/PacketReader.cpp
        source/network/serialization/PacketWriter.cpp
        include/network/serialization/PacketWriter.h
        include/network/packets/types.h
        include/network/serialization/write_helper.h
        include/network/serialization/read_helper.h
        source/network/SessionManager.cpp
        include/network/SessionManager.h
        source/network/Server.cpp
        include/network/Server.h)
