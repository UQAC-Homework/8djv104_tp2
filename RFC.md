# RFC

## Résumé

ce que fait le protocole, en quelques lignes

## Conventions

référence à la RFC 2119, ordre des octets, notation utilisée

## Terminologie

définition des termes employés (connexion, pair, paquet de données, paquet de contrôle, etc.).

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

The field `Type` **MUST NOT** be `0x0`. A packet with the type an invalid type **SHOULD** be ignored and **MUST** be
discarded.

### `CON`

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

The field `Length` is offset by 1. For a username of length `1`, the length will be `0x0`. This allows the
username to be a maximum length of `16`, while still fitting inside a nybble. Usernames of length `0` **MUST NOT** be
allowed.

The field `Username` is written on alternating bytes. The first nybble of a character are written as the lower nybble of
the first byte, and the last nybble of a character are written as the upper nybble of the second byte.

#### Examples

Example of a packet:

```
01                 Type
55                 Length + First nybble of Username
06 c6 17 96 57     Username
20                 Last nybble of Username + Unused
```

Example of how the username "Player" is found:

```
Raw: 01 55 06 c6 17 96 57 20
Nybbles: 0 1 5 5 0 6 c 6 1 7 9 6 5 7 2 0
Grouped Nybbles:
	- Type: 0 1
	- Length: 5
	- Username: 5 0 6 c 6 1 7 9 6 5 7 2
	- Unused: 0

Type: 1
Length: 5 -> 6
Username:
	5 0 6 c 6 1 7 9 6 5 7 2
	50 6c 61 79 65 72
	P  l  a  y  e  r
```

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