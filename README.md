<img src="image/logo.png">


## Port Snippet 📔

PortSnippet monitors source codes and automatically generates snippets!

Available only in VS Code✌

## Demo 📸

<img src = "image/demo.gif">


## How to use 💻

Put meta tags between your code that you want to save as a snippet!

### Meta Tags

```cpp
// #PORT#
// name: ""
// prefix: ""
// description: ""

```

```cpp
// #PORT_END 
```

PortSnippet detects these meta tags to find `#PORT#` and `#PORT_END#` using regex.


### Example

```cpp
// #PORT#
// name: "SegmentTree"
// prefix: "seg"
// description: "セグ木"

template <class S, S (*op)(S, S), S (*e)()> struct segtree {
    segtree() : segtree(0) {}

    ...

    void update(int k) { d[k] = op(d[2 * k], d[2 * k + 1]); }
};

// #PORT_END

```

## Config

You need put a config file on the same file as PortSnippet.

```json 
{
    "snippets_dir": "",
    "dirs": [
        ""
    ],
    "files": [
        ""
    ]
}
```

`"snippets_dir"` depends on your platform. (check [this](https://vscode-docs.readthedocs.io/en/stable/customization/userdefinedsnippets/))

- Windows:  `%APPDATA%\Code\User\snippets`
- Mac `$HOME/Library/Application Support/Code/User/snippets`
<!-- - Linux $HOME/.config/Code/User/snippets/(language).json -->

**※ Write an ABSOLUTE path！ ※**

<br>

`"dirs"` and `"files"` means files or directories that you want to monitor.

When you change the files that PortSnippet's monitoring, it detects any changes of these files and automatically generate a snippet.

After modifing the config file, make sure to restart PortSnippet! (check [#Arguments](#Arguments))

<br>

## Arguments







## How it works






## Contribute

We would love you for the contribution to **PortSnippet**, check the ``LICENSE`` file for more info.


## Others

Yuiga Wada -  [WebSite](https://yuiga.dev)
Twitter         - [@YuigaWada](https://twitter.com/YuigaWada)





Distributed under the MIT license. See ``LICENSE`` for more information.

[https://github.com/YuigaWada/PortSnippet](https://github.com/YuigaWada/PortSnippet)
