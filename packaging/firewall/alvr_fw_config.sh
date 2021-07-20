#!/usr/bin/env bash
# Basic script to add / remove firewall configuration for ALVR
# Usage: ./alvr_fw_config.sh firewalld|iptables|ufw add|remove
# Exit codes: 
# 1 - Invalid command
# 2 - Invalid action
# 99 - Feature not implemented

iptables_cfg() {
    exit 99
}

firewalld_cfg() {
    for zone in $(firewall-cmd --get-active-zones | grep -P '^\w+.*\w$'); do 
        if [ "${1}" == 'add' ]; then
            if ! firewall-cmd --zone="${zone}" --list-services | grep 'alvr' >/dev/null 2>&1; then
                firewall-cmd --zone="${zone}"  --add-service='alvr'
            fi
            if ! firewall-cmd --zone="${zone}" --list-services --permanent | grep 'alvr' >/dev/null 2>&1; then
                firewall-cmd --zone="${zone}"  --add-service='alvr' --permanent
            fi
        elif [ "${1}" == 'remove' ]; then
            if firewall-cmd --zone="${zone}" --list-services | grep 'alvr' >/dev/null 2>&1; then
                firewall-cmd --zone="${zone}"  --remove-service='alvr'
            fi
            if firewall-cmd --zone="${zone}" --list-services --permanent | grep 'alvr' >/dev/null 2>&1; then
                firewall-cmd --zone="${zone}"  --remove-service='alvr' --permanent
            fi
        else
            exit 2
        fi
    done
}

ufw_cfg() {
    exit 99
}

main() {
    case "${1,,}" in
        'firewalld') firewalld_cfg "${2,,}";;
        'iptables') iptables_cfg "${2,,}";;
        'ufw') ufw_cfg "${2,,}";;
        *) exit 1
    esac
}

main "${@}"