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

### `CON`

Packet sent by the client to the server to request a connection.

#### Format

```
 0      7 8     11 12     139
+--------+--------+----------+
|  Type  | Length | Username |
+--------+--------+----------+
```

#### Fields

| Name     | Size     | Description                                                                                                         |
|----------|----------|---------------------------------------------------------------------------------------------------------------------|
| Type     | 8 bits   | Magic number used to define the type of the packet. **MUST** be `0x01`.                                             |
| Length   | 4 bits   | Length of the username in bytes. **MUST** be offset by 1. A length of `1` is `0x0` while a length of `16` is `0xE`. |
| Username | 128 bits |                                                                                                                     |

The field `Length` is offset by 1. For an username of length `1`, the length will be `0x0`. This allows the username to be of length `16`, while still fitting inside 4 bits. This isn't an issue, since usernames of length `0` are not allowed.

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