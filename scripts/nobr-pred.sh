#!/bin/bash

# 0xc00110e3, bit 1 is "suppress non-branch predictions" on Zen2

if [[ -z ${1} ]]; then echo "usage: ${0} [off|on]"; exit -1; fi
if [[ $EUID != 0 ]]; then echo "must be root"; exit -1; fi

prev=$(sudo rdmsr -c -p 15 0xc00110e3)

if [[ ${1} == "off" ]]; then
	next=$( printf '0x%x' $(( ${prev} & ~(1 << 1) )) )
elif [[ ${1} == "on" ]]; then
	next=$( printf '0x%x' $(( ${prev} | (1 << 1) )) )
else
	echo "usage: ${0} [off|on]"
	exit -1
fi
echo "Writing 0xc00110e3: ${prev} => ${next} for core 15"
sudo wrmsr -p 15 0xc00110e3 ${next}


