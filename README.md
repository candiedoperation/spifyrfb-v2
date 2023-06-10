# SpifyRFB
A Modern RFB Server implementation using Rust, written from scratch.

### Platforms

| Name                       | SpifyRFB Support |
|----------------------------|--------------|
| Windows                    | ✅            |
| Linux (X11)                | ✅            |
| macOS                      |              |

### Security Types (RFB Protocol)

|Name               |Number      | SpifyRFB Support |
|-------------------|------------|--------------|
|None               |          1 |           ✅ |
|VNC Authentication |          2 |              |

### Encodings (RFB Protocol)

| Name     | Number | SpifyRFB Support | 
|----------|--------|--------------|
| Raw      | 1      |        ✅     |
| CopyRect | 2      |              |
| RRE      | 3      |              |
| Hextile  | 5      |        ✅      |
| TRLE     | 15     |              | 
| ZRLE     | 16     |              |


### Transports

| Name                       | SpifyRFB Support |
|----------------------------|--------------|
| TCP Sockets                | ✅            |
| Websockets                 |              |
| Encrypted Websockets       |              |
