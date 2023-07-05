#!/usr/bin/expect -f
set timeout 1200
set address1 [lindex $argv 0]
set address2 [lindex $argv 1]
set gateway [lindex $argv 2]
set hostname [lindex $argv 3]
set mac1 [lindex $argv 4]
set mac2 [lindex $argv 5]
set pidfile [lindex $argv 6]

spawn sudo qemu-system-arm \
    -M virt \
    -m 512 \
    -kernel image/openwrt-armvirt-32-zImage \
    -no-reboot -nographic \
    -device virtio-net,netdev=net0,mac=$mac1 \
    -netdev bridge,id=net0,br=br0 \
    -device virtio-net,netdev=net1,mac=$mac2 \
    -netdev bridge,id=net1,br=br0 \
    -pidfile $pidfile \
    -snapshot \
    -drive file=image/openwrt-armvirt-32-rootfs-ext4.qcow2,format=qcow2,if=virtio \
    -append "root=/dev/vda rootwait"

# expect "Please press Enter to activate this console."
expect "br-lan: link becomes ready"
sleep 5 

send "\r"
send "uci set network.lan.ipaddr='$address1'\r"
send "uci set network.lan.netmask='255.255.255.0'\r"
send "uci set network.wan=interface\r"
send "uci set network.wan.ifname='eth1'\r"
send "uci set network.wan.proto='static'\r"
send "uci set network.wan.ipaddr='$address2'\r"
send "uci set network.wan.dns='8.8.8.8'\r"
send "uci set network.wan.netmask='255.255.255.0'\r"
send "uci set network.wan.gateway='$gateway'\r"

send "uci commit network\r"
send "/etc/init.d/network restart\r"

send "uci set system.@system\[0\].hostname='$hostname'\r"
send "uci commit system\r"
send "/etc/init.d/system reload\r"
send "/etc/init.d/firewall stop\r"

# run forever
while {1} {
    sleep 1000000
}
