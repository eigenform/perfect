#!/bin/bash

if [[ -z $1 ]]; then echo "usage: $0 [on|off]"; exit -1; fi
if [[ $EUID != 0 ]]; then echo "Must be root"; exit -1; fi

if [[ ${1} == "on" ]]; then 
	sysctl vm.mmap_min_addr=0
elif [[ ${1} == "off" ]]; then 
	sysctl vm.mmap_min_addr=65536
else
	echo "usage: $0 [off|on]"
	exit -1
fi
