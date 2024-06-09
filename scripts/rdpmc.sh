#!/bin/bash

if [[ -z $1 ]]; then echo "usage: $0 [on|off]"; exit -1; fi
if [[ $EUID != 0 ]]; then echo "Must be root"; exit -1; fi

RDPMC_CTRL=/sys/bus/event_source/devices/cpu/rdpmc

if [[ ${1} == "on" ]]; then 
	echo 2 > ${RDPMC_CTRL}
elif [[ ${1} == "off" ]]; then 
	echo 1 > ${RDPMC_CTRL}
else
	echo "usage: $0 [off|on]"
	exit -1
fi
