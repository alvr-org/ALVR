#!/usr/bin/env bash
# Basic script to add / remove firewall configuration for ALVR
# Usage: ./alvr_fw_config.sh add|remove
# Exit codes:
# 1 - Invalid command
# 2 - Invalid action
# 3 - Failed to copy UFW configuration
# 99 - Firewall not found
# 126 - pkexec failed - Request dismissed

firewalld_cfg() {
    # Iterate around each active zone
    for zone in $(firewall-cmd --get-active-zones | grep -P '^\w+.*\w$'); do
        if [ "${1}" == 'add' ]; then
            # If running or permanent alvr service is missing, add it
            if ! firewall-cmd --zone="${zone}" --list-services | grep 'alvr' >/dev/null 2>&1; then
                firewall-cmd --zone="${zone}" --add-service='alvr'
            fi
            if ! firewall-cmd --zone="${zone}" --list-services --permanent | grep 'alvr' >/dev/null 2>&1; then
                firewall-cmd --zone="${zone}" --add-service='alvr' --permanent
            fi
        elif [ "${1}" == 'remove' ]; then
            # If running or persistent alvr service exists, remove it
            if firewall-cmd --zone="${zone}" --list-services | grep 'alvr' >/dev/null 2>&1; then
                firewall-cmd --zone="${zone}" --remove-service='alvr'
            fi
            if firewall-cmd --zone="${zone}" --list-services --permanent | grep 'alvr' >/dev/null 2>&1; then
                firewall-cmd --zone="${zone}" --remove-service='alvr' --permanent
            fi
        else
            exit 2
        fi
    done
}

ufw_cfg() {
    # Try and install the application file
    if ! ufw app info 'alvr'; then
        # Pull application file from local build first if the script lives inside it
        if [ -f "$(dirname "$(realpath "${0}")")/ufw-alvr" ]; then
            cp "$(dirname "$(realpath "${0}")")/ufw-alvr" '/etc/ufw/applications.d/'
        elif [ -f '/usr/share/alvr/ufw-alvr' ]; then
            cp '/usr/share/alvr/ufw-alvr' '/etc/ufw/applications.d/'
        else
            exit 3
        fi
    fi

    if [ "${1}" == 'add' ] && ! ufw status | grep 'alvr' >/dev/null 2>&1; then
        ufw allow 'alvr'
    elif [ "${1}" == 'remove' ] && ufw status | grep 'alvr' >/dev/null 2>&1; then
        ufw delete allow 'alvr'
    else
        exit 2
    fi
}

iptables_cfg() {
    first_port_match_count=$(iptables -S | grep -c '9943')
    second_port_match_count=$(iptables -S | grep -c '9944')
    if [ "${1}" == 'add' ]; then
        if [ "$first_port_match_count" == "0" ] || [ "$second_port_match_count" == "0" ]; then
            if [ ! -d '/etc/iptables' ]; then
                mkdir '/etc/iptables'
            fi

            iptables -I OUTPUT -p tcp --sport 9943 -j ACCEPT
            iptables -I INPUT -p tcp --dport 9943 -j ACCEPT
            iptables -I OUTPUT -p udp --sport 9943 -j ACCEPT
            iptables -I INPUT -p udp --dport 9943 -j ACCEPT
            iptables -I OUTPUT -p tcp --sport 9944 -j ACCEPT
            iptables -I INPUT -p tcp --dport 9944 -j ACCEPT
            iptables -I OUTPUT -p udp --sport 9944 -j ACCEPT
            iptables -I INPUT -p udp --dport 9944 -j ACCEPT
            iptables-save >/etc/iptables/rules.v4
        fi
    elif [ "${1}" == 'remove' ]; then
        if [ "$first_port_match_count" == "4" ] || [ "$second_port_match_count" == "4" ]; then
            iptables -D OUTPUT -p tcp --sport 9943 -j ACCEPT
            iptables -D INPUT -p tcp --dport 9943 -j ACCEPT
            iptables -D OUTPUT -p udp --sport 9943 -j ACCEPT
            iptables -D INPUT -p udp --dport 9943 -j ACCEPT
            iptables -D OUTPUT -p tcp --sport 9944 -j ACCEPT
            iptables -D INPUT -p tcp --dport 9944 -j ACCEPT
            iptables -D OUTPUT -p udp --sport 9944 -j ACCEPT
            iptables -D INPUT -p udp --dport 9944 -j ACCEPT
            iptables-save >/etc/iptables/rules.v4
        fi
    else
        exit 2
    fi
}

main() {
    # If we're not root use pkexec for GUI prompt
    if [ "${USER}" == 'root' ]; then
        # Check if firewall-cmd exists and firewalld is running
        if which firewall-cmd >/dev/null 2>&1 && firewall-cmd --state >/dev/null 2>&1; then
            firewalld_cfg "${1,,}"
        # Check if ufw exists and is running
        elif which ufw >/dev/null 2>&1 && ! ufw status | grep 'Status: inactive' >/dev/null 2>&1; then
            ufw_cfg "${1,,}"
        elif which iptables >/dev/null 2>&1; then
            iptables_cfg "${1,,}"
        else
            exit 99
        fi
    else
        pkexec "$(realpath "${0}")" "${@}"
    fi
}

main "${@}"
