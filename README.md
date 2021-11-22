# vts-set-param

> ⚠️ Please see [vtubestudio-cli](https://github.com/walfie/vtubestudio-cli)
> for a more general-purpose CLI tool.

CLI tool to register/set a VTube Studio custom parameter in a one-shot manner.
The intent is for it to be callable by scripts that just want to set a
parameter once, and don't need to maintain a persistent websocket connection.

## Example

```sh
vts-set-param --param-id MyCustomParam --value 5
```

The above command will connect to the VTube Studio API via the default
websocket address (`ws://localhost:8001`), register a custom parameter
`MyCustomParam`, set its value to `5`, and then disconnect.

It will check for an existing authentication token in the config file
`~/.config/vts-set-param.json`, and if the file doesn't exist (or the token is
invalid), will request a new token, and save it in the config file
for future invocations.

Since VTube Studio will reset a custom param if it hasn't received data for it
after some timeout, a way to persist the value is to set it as the default:

```sh
vts-set-param --param-id MyCustomParam --default 5
```

## Usage

Additional flags can be set to customize the parameter's min/max values, and
set plugin metadata used in the initial authentication flow.

```
vts-set-param 0.1.0

USAGE:
    vts-set-param [OPTIONS] --param-id <param-id>

FLAGS:
        --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --default <default>                       [default: 0]
        --explanation <explanation>
    -h, --host <host>                             [default: localhost]
        --max <max>                               [default: 100]
        --min <min>                               [default: 0]
        --param-id <param-id>
        --plugin-developer <plugin-developer>     [default: Walfie]
        --plugin-name <plugin-name>               [default: vts-set-param]
    -p, --port <port>                             [default: 8001]
        --token <token>                           [env: TOKEN]
        --value <value>
```

