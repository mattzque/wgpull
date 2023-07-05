apt remove -y --purge netplan.io || true
apt remove -y --purge snapd || true

hostnamectl set-hostname ubuntu || true

dbus-uuidgen > /etc/machine-id

systemctl enable systemd-networkd || true

cat << EOF > /etc/systemd/network/01-ens3.network
[Match]
Name=ens3
[Network]
Address=10.180.0.2/24
DNS=8.8.8.8
EOF

# we set a static ip based on the mac address to make this easy
# we also set the hostname prefixed by the last ip octet
cat << EOF > /root/setup_ip.sh
#!/bin/bash
MAC=\$(cat /sys/class/net/ens3/address)
IFS=":" read -ra MAC_PARTS <<< "\$MAC"
IP1=\$(printf '%d' 0x\${MAC_PARTS[2]})
IP2=\$(printf '%d' 0x\${MAC_PARTS[3]})
IP3=\$(printf '%d' 0x\${MAC_PARTS[4]})
IP4=\$(printf '%d' 0x\${MAC_PARTS[5]})
IP="\$IP1.\$IP2.\$IP3.\$IP4"
GATEWAY="\$IP1.\$IP2.\$IP3.1"
hostnamectl set-hostname ubuntu\$IP4
echo "[Match]
Name=ens3
[Network]
Address=\${IP}/24
Gateway=\${GATEWAY}
DNS=8.8.8.8" > /etc/systemd/network/01-ens3.network
systemctl restart systemd-networkd
EOF
chmod 755 /root/setup_ip.sh

cat << EOF > /etc/systemd/system/setup-ip.service
[Unit]
Description=Set static IP

[Service]
Type=oneshot
ExecStart=/bin/bash /root/setup_ip.sh

[Install]
WantedBy=multi-user.target
EOF
chmod 644 /etc/systemd/system/setup-ip.service
systemctl daemon-reload
systemctl enable setup-ip

mkdir -p /home/admin/.ssh || true
chmod 700 /home/admin/.ssh || true
chown admin:admin /home/admin/.ssh || true
echo "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFJwmYluHkx0tKTJKExCh+LI7jcoI/DvZhyDE8Uw9en3" >> /home/admin/.ssh/authorized_keys
chown admin:admin /home/admin/.ssh/authorized_keys || true
chmod 600 /home/admin/.ssh/authorized_keys || true
