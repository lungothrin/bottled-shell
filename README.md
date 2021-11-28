# bottled-shell: Run systemd in WSL2

Run systemd with namespace in WSL2. Inspired by [subsystemctl](https://github.com/sorah/subsystemctl)

## Difference from other solutions

- Launch systemd-enabled shell from start menu
- Support Visual Studio Code(Remote - WSL) extension

## Install

1. Install package

Download installer from [Release Page](https://github.com/lungothrin/bottled-shell/releases), execute it with root privilege.

```bash
sudo bash installer-vx.x.x.sh 
```

2. Set your login shell to  `bottled-shell`

Suppose you are using `bash` as login shell.

Create an alias for bottled-shell.

```bash
sudo ln -s /opt/bottled-shell/bin/bottled-shell /opt/bottled-shell/bin/bottled-bash
```

Edit `/etc/passwd`, set your login shell to `bottled-bash`.

```
username:x:1000:1000::/home/username:/opt/bottled-shell/bin/bottled-bash
```

3. All done.

Try open a shell from start menu, or open a terminal in VSCode.