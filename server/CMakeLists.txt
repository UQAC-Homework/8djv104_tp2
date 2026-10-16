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
        include/network/serialization/PacketReader.h
        source/network/serialization/PacketReader.cpp
        source/network/serialization/PackerWriter.cpp
        include/network/serialization/PackerWriter.h
        include/network/packets/types.h
        include/network/serialization/WriteHelper.h
        source/network/serialization/WriteHelper.cpp
        include/network/serialization/ReadHelper.h
        source/network/serialization/ReadHelper.cpp)
