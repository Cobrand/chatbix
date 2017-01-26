# Chatbix, the Chatbox with a typo

## Build

```sh
$ echo 'DATABASE_URL=postgres://dbuser:dbpassword@dbaddress/chatbox' > .env
$ echo 'LISTEN_URL=0.0.0.0:PORT' >> .env
$ rustup override nightly
$ cargo run --bin chatbix
```

## API

Every route below has for base URI `http(s)://address.of.chat/api/`

### Structs and objects

#### Timestamp

Timestamps can either be of the format 1485402097 or of the format
2017-01-26T04:41:37 (see iso 8601 without the Z at the end)

#### Tags

tags are on 32bits:

* tags & 1 = logged\_in (generated by the server)
* tags & 2 >> 1 = generated // is this message generated from another message
* tags & 4 >> 2 = bot // is this message sent by a bot ?
* tags & 8 >> 3 = no\_notif // should this message ignore notifications rules and not notify him
* tags & (2^4 + 2^5 + 2^6 + 2^7) >> 4 = show\_value
* everything else: reserved for later use
  
`show_value`:

    * show\_value : u4; 
    * show\_value = 0: no change;
    * show\_value = 1 to 4; 'hidden' message, with 4 being more hidden than 1
    * show\_value = 9 to 12; 'important' message, with 12 being more important
    
    ^ these are basically ignored by the server and are only implementation dependant
    you can set up your client to never show show_value = 4, and show show_value = 1 but not
    notify, ...

#### Auth Key

Auth Key describes a alphanumeric string of length 16, for instance "f4xVbuTlR9bTl0i6"

It is retrieved when logging in, and can be used to confirm one's identity (to be sure that
messages come from the owner and not some fake). It can also be used to have access to
admin commands, but this happens only when the user is an admin (of course).

### Retrieving messages

Method: GET

* Retrieving the last X messages : `/api/get_messages`
  (Note: X is hardcoded for now)
* Retrieving all messages since T : `/api/get_messages?timestamp=T`
* Retrieving all messages between T1 and T2 `/api/get_messages?timestamp=T1&timestamp_end=T2`
* Retrieving all messages of the default channel plus the channel C `/api/get_messages?channel=C`
* Retrieving all messages of the default channel plus multiple channels C1, C2, ... : `/api/get_messages?channel=C1?channel=C2`, `/api/get_messages?channels=C1,C2,C3`, or any combination of both
* You can retrieve all messages until date T2 as well, but beware, the load might be huge if thousands of messages are in the DB ...
* If you want to only retrieve a channel without the default one: `/api/get_messages?channel=C?no_default_channel?timestamp=T`

### Sending a new message

The URI is always POST `/api/new_message`

The body must always be valid JSON.

These values are required:

* author: string
* content: string

These values are optional:

* tags: integer, see [the tags section](#Tags)
* color: a value of "#RRGGBB" is expected, but it is not checked. You could input "red", "blue" or whatever as well, but don't expect it to be parsed by other clients
* channel: string, name of the channel this should be sent to
* auth\_key: string, see the section Auth Key

### Logging in

POST `/api/login`

Required values in the JSON body:

* username: string
* password: string

your password is sha512'd before entering the DB, so you can either input a hashed password or the raw password.
Default clients will sha512 the password before sending it via this request though, so take that into account
if you want to be compatible with other clients.

Returns `{"auth_key":AUTH_KEY}` on success

The AUTH\_KEY will stay the same until the server is restarted (and thus the cache is discarded), or until you
call `/api/logout`. This means that multiple clients can be connected with the same auth\_key.

### Logging out

POST `/api/logout`

Required values:

* username: string
* auth\_key: see Auth Key

### Registering

POST `/api/register`

Required values in the JSON body:

* username: string
* password: string

Returns `{"auth_key":AUTH_KEY}` on success


## License

Dual licensed under MIT / Apache-2.0
