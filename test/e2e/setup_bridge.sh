INTERFACE=enp7s0
BRIDGE=br0

sudo sysctl -w net.ipv4.ip_forward=1
sudo iptables -t nat -A POSTROUTING -o $INTERFACE -j MASQUERADE
sudo iptables -A FORWARD -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT
sudo iptables -A FORWARD -i $BRIDGE -o $INTERFACE -j ACCEPT
sudo iptables -I FORWARD -i $BRIDGE -o $BRIDGE -j ACCEPT
