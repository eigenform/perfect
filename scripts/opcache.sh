#!/bin/bash

# 0xc0011021, bit 5 is "opcache disable" on Zen2

if [[ -z ${1} ]]; then echo "usage: ${0} [off|on]"; exit -1; fi
if [[ $EUID != 0 ]]; then echo "must be root"; exit -1; fi

prev=$(sudo rdmsr -c -p 15 0xc0011021)

if [[ ${1} == "on" ]]; then
	next=$( printf '0x%x' $(( ${prev} & ~(1 << 5) )) )
elif [[ ${1} == "off" ]]; then
	next=$( printf '0x%x' $(( ${prev} | (1 << 5) )) )
else
	echo "usage: ${0} [off|on]"
	exit -1
fi
echo "Writing 0xc0011021: ${prev} => ${next} for core 15"
sudo wrmsr -p 15 0xc0011021 ${next}


