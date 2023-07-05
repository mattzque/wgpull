#!/bin/bash
# starts an ubuntu vm in qemu that gets assigned the given IP address
# the IP address is passed into the vm via the mac address of the nic
# the last 4 octets of the mac address are used to represent the IP.
# the gateway is the same except for 1 as the last octet.
# TODO: A better solution would be to use -append to add some boot parameters
#       and then parse /proc/cmdline to assign the passed IP/gateway
IP=$1
pidfile=$2

IFS="." read -ra ip_parts <<< "$IP"
mac1=$(printf '%02x' ${ip_parts[0]})
mac2=$(printf '%02x' ${ip_parts[1]})
mac3=$(printf '%02x' ${ip_parts[2]})
mac4=$(printf '%02x' ${ip_parts[3]})

MAC="52:54:$mac1:$mac2:$mac3:$mac4"
echo $MAC

sudo /usr/bin/qemu-system-x86_64 \
    -enable-kvm \
    -m 512 \
    -smp 2 \
    -nographic \
    -display none \
    -pidfile $pidfile \
    -drive file=./ubuntu_20_04_amd64/ubuntu_20_04_amd64.qcow2,format=qcow2,if=virtio \
    -snapshot \
    -net nic,model=virtio,macaddr=$MAC \
    -net bridge,br=br0
    
# < /dev/null > /dev/null 2>&1 &

#PID=$!
#echo "qemu instance: $PID"
#trap 'sudo pkill -P "$PID"' SIGINT
#wait "$PID"