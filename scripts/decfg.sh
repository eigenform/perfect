#!/bin/bash

# In the abscence of the appropriate microcode patch level, Linux mitigates
# Zenbleed by setting DE_CFG[9] when cores come online. This script will let
# you set/unset this bit on all cores. 

if [[ -z ${1} ]]; then echo "usage: ${0} [off|on]"; exit -1; fi
if [[ $EUID != 0 ]]; then echo "must be root"; exit -1; fi

nproc=$(( $(nproc) ))

for id in $(seq 0 $nproc); do
	prev=$(sudo rdmsr -c -p ${id} 0xc0011029)

	if [[ ${1} == "off" ]]; then
		next=$( printf '0x%x' $(( ${prev} & ~(1 << 9) )) )
	elif [[ ${1} == "on" ]]; then
		next=$( printf '0x%x' $(( ${prev} | (1 << 9) )) )
	else
		echo "usage: ${0} [off|on]"
		exit -1
	fi
	echo "Writing 0xc0011029: ${prev} => ${next} for core ${id}"
	sudo wrmsr -p ${id} 0xc0011029 ${next}

done

