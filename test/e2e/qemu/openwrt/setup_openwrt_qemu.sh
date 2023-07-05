#!/usr/bin/expect -f
set timeout 1200
set address [lindex $argv 0]
set gateway [lindex $argv 1]
set hostname [lindex $argv 2]
set filename "../../ssh_key.pub"
set fp [open $filename r]
set sshkey [read $fp]
close $fp

spawn sudo qemu-system-arm \
    -M virt \
    -m 512 \
    -kernel image/openwrt-armvirt-32-zImage \
    -no-reboot -nographic \
    -net nic,model=virtio \
    -net bridge,br=br0 \
    -drive file=image/openwrt-armvirt-32-rootfs-ext4.qcow2,format=qcow2,if=virtio \
    -append "root=/dev/vda rootwait"

expect "br-lan: link becomes ready"
send_user "\nPlease wait to setup OpenWRT Image ... (this can take a 1-3 minutes)\n"
sleep 5 

send "\r"
send "uci set network.lan.ipaddr='$address'\r"
send "uci set network.lan.gateway='$gateway'\r"
send "uci set network.lan.dns='8.8.8.8'\r"
send "uci commit network\r"
send "uci set system.@system\[0\].hostname='$hostname'\r"
send "uci commit system\r"
send "/etc/init.d/network restart\r"
sleep 5 

send "opkg update\r"
send "opkg install openssh-sftp-server kmod-wireguard wireguard-tools\r"

send "mkdir -p /root/.ssh\r"
send "chmod 700 /root/.ssh\r"
send "echo \"$sshkey\" >> /root/.ssh/authorized_keys\r"
send "chmod 600 /root/.ssh/authorized_keys\r"
send "touch /root/openwrt_ready\r"

send "poweroff\r"
sleep 60
