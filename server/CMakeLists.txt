cmake_minimum_required(VERSION 4.3)
project(server)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

add_executable(server
        main.cpp
        include/socket.h
        include/PacketType.h
        source/Logger.cpp
        include/Logger.h
        include/serialization/PacketReader.h
        source/serialization/PacketReader.cpp
        source/serialization/PackerWriter.cpp
        include/serialization/PackerWriter.h
        include/network/packets/types.h
        include/serialization/WriteHelper.h
        source/serialization/WriteHelper.cpp)
