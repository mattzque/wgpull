# E2E Testing for wgpull

The e2e tests use qemu to simulate a network of machines connected with Wireguard
managed by wgpull. Right now it tests with:

* Ubuntu 20.21 `x86_64`
* OpenWRT 22.03.5 

It tests the release packages (.deb and .ipk).

The test suite itself is written in Python with PyTest.
To build the Ubuntu image, Packer with cloud-init is used.

If you wish to run the e2e tests you need to install some dependencies first,
and setup a bridge interface, the tests spawn qemu using sudo.

## Requirements

* QEMU: qemu-kvm qemu-system-x86 qemu-system-arm qemu-utils
* Packer: https://www.hashicorp.com/products/packer (see: https://www.hashicorp.com/official-packaging-guide)
* Expect
* Python / PyTest (install pytests/requirements.txt)

## QEMU Network Bridge

```
/etc/systemd/network/br0.network
[Match]
Name=br0

[Network]
Address=10.180.0.1/24
IPForward=yes
IPMasquerade=ipv4
```

```
/etc/systemd/network/br0.netdev
[NetDev]
Name=br0
Kind=bridge
```

Firewall Configuration:

Replace eth0 with public facing interface.

```   
sudo sysctl -w net.ipv4.ip_forward=1
sudo iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE
sudo iptables -A FORWARD -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT
sudo iptables -A FORWARD -i br0 -o eth0 -j ACCEPT
# this might be necessary depending on the host setup: see this issue:
#  https://serverfault.com/questions/963759/docker-breaks-libvirt-bridge-network
sudo iptables -I FORWARD -i br0 -o br0 -j ACCEPT
```   

## Prepare images

```
make prepare
```

## Run Tests

```
make test
```