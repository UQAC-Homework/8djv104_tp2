# RFC

## Résumé

ce que fait le protocole, en quelques lignes

## Conventions

This section will go over the conventions used in this document. It will define how a given value is written.

### Numbers

- **Binary**: Any value with the prefix `0b` is displayed as a binary value.
- **Hexadecimal**: Any value with the prefix `0x` is displayed as a hexadecimal value.
- **Decimal**: Any value without a prefix is displayed as a decimal value.
- **Numeration**: Any single digit decimal number written into text is displayed as their spelled out counter-part, succeeded by their decimal representation written within parentheses. 
- **Calculated Number**: Any value resulting from a calculation is displayed as code.

### Binary arrangement

The protocol uses the [little-endian](https://en.wikipedia.org/wiki/Endianness) format.

référence à la RFC 2119, ordre des octets, notation utilisée



## Terminology

This section will go over the terminology used in this document. It will define what certain terms mean in the context of this document.

- **MUST**: Defines a behavior that is required to be implemented. The protocol will not work properly if the behavior is missing or altered.
- **MUST NOT**: Defines a behavior that is required to be missing. The protocol will not work properly if a behavior handles the specific situation.
- **SHOULD**: Defines a suggested behavior. The protocol will work if a behavior is implemented or not. However, the protocol assumes that the suggested behavior is the one used. A custom behavior will not be able to be used by the protocol.
- **COULD**: Defines a potential behavior. The protocol will work if a behavior is implemented or not. The protocol will assume that no implementation was defined, and the behavior will be ignored.
- **Nybble**: Defines a group of four (4) consecutive bits.
- **Upper Nybble**: Defines the first half of a byte. For the byte `0x2A`, the upper nybble is `0x2`.
- **Lower Nybble**: Defines the second half of a byte. For the byte `0x2A`, the lower nybble is `0xA`.

définition des termes employés (connexion, pair, paquet de données, paquet de contrôle, etc.).

## Technical Aspects

This section goes over the technical aspects of the protocol. It will define how, 

## Vue d’ensemble

fonctionnement général, rôles du client et du serveur.

## Format des paquets

en-tête commun et format détaillé de chaque type de paquet (diagramme + description de chaque champ).

### Shared Common header

This section defines the shared common header between all packets. All packets **MUST** start with the header. Each
packet is subjected to the rules of this header.

#### Format

```
 0      7
+--------+
|  Type  |
+--------+
```

#### Fields

| Name | Size             | Description                                         |
|------|------------------|-----------------------------------------------------|
| Type | 8 bits, unsigned | Magic number used to define the type of the packet. |

The field `Type` **MUST NOT** be `0x0`. A packet with an unsupported type **SHOULD** be ignored.

### `CONNECTION_REQUEST`

Packet sent by the client to the server to request a connection.

#### Format

```
 0      7 8     11 12     139 140  143
+--------+--------+----------+--------+
|  Type  | Length | Username | Unused |
+--------+--------+----------+--------+
```

#### Fields

| Name     | Size               | Description                                               |
|----------|--------------------|-----------------------------------------------------------|
| Type     | 8 bits, unsigned   | Field shared with the common header. **MUST** be `0x01`.  |
| Length   | 4 bits, unsigned   | Length of the username in bytes. **MUST** be offset by 1. |
| Username | 128 bits, unsigned | Characters of the username. **MUST** be ASCII characters. |
| Unused   | 4 bits             | Unused bits                                               |

The field `Length` is offset by one (1). For a username of one (1) character, the length **MUST** be `0x0`. For a username of 16 characters, the length **MUST** be `0xF`. This allows the
username to be a maximum of 16 characters, while still fitting inside a nybble.

The field `Username` **MUST** be of length defined by the field `Length`. The protocol **MUST NOT** assume that the field takes the full possible length. For a username of three (3) characters, the total size of the field `Username` will be `24` bits. Usernames with no character or a length of zero (0) or less **MUST NOT** be allowed.

Each character of the field `Username` **MUST** be written on two (2) bytes. The first nybble of the character **MUST** be written on the lower nybble of the first byte. The second nybble of the character **MUST** be written on the upper nybble of the second byte.

The last nybble of the packet is unused. The protocol has no behavior logic defined for that data. The client **COULD** choose to set it, but **MUST NOT** assume that the server will read it. The server **SHOULD** ignore it.

#### Examples

Example of a packet:

```
01                 Type
55                 Length + Upper nybble of Username
06 c6 17 96 57     Username
20                 Lower nybble of Username + Unused
```

Example of how the username `Player` is found:

```
Bytes: 01 55 06 c6 17 96 57 20
Nybbles: 0 1 5 5 0 6 c 6 1 7 9 6 5 7 2 0
Grouped Nybbles:
	- Type: 0 1
	- Length: 5
	- Username: 5 0 6 c 6 1 7 9 6 5 7 2
	- Unused: 0

Type: 1
Length: 5
Username:
	5 0 6 c 6 1 7 9 6 5 7 2
	|/  |/  |/  |/  |/  |/
	50  6c  61  79  65  72
	P   l   a   y   e   r
```

### `CONNECTION_ACCEPTED`

Packet sent by the server to the client to notify that the connection request was accepted.

#### Format

```
 0      7 
+--------+
|  Type  |
+--------+
```

#### Fields

| Name     | Size               | Description                                               |
|----------|--------------------|-----------------------------------------------------------|
| Type     | 8 bits, unsigned   | Field shared with the common header. **MUST** be `0x02`.  |

### `CONNECTION_REJECTED`

Packet sent by the server to the client to notify that the connection request was rejected.

#### Format

```
 0      7 
+--------+
|  Type  |
+--------+
```

#### Fields

| Name     | Size               | Description                                               |
|----------|--------------------|-----------------------------------------------------------|
| Type     | 8 bits, unsigned   | Field shared with the common header. **MUST** be `0x03`.  |

## Machines à états

une pour le client, une pour le serveur (vu d’une connexion). Diagramme et table de transitions.

## Procédures

établissement, terminaison, heartbeat, numérotation, NACK, retransmission.

## Temporisateurs et constantes

tableau récapitulatif de toutes les valeurs.

## Gestion des erreurs

paquets malformés, inconnus, dupliqués, hors connexion, etc.

## Exemples d’échanges

au moins trois diagrammes de séquence, dont un avec perte de paquets.