#!/bin/bash

if [[ -z $1 ]]; then echo "usage: $0 [on|off]"; exit -1; fi
if [[ $EUID != 0 ]]; then echo "Must be root"; exit -1; fi

# Enable RDPMC usage
RDPMC_CTRL=/sys/bus/event_source/devices/cpu/rdpmc
# We want the frequency to be stable!
BOOST_CTRL=/sys/devices/system/cpu/cpufreq/boost
# We want the frequency to be stable!!!
GOV_CTRL=/sys/devices/system/cpu/cpufreq/policy0/scaling_governor

if [[ ${1} == "on" ]]; then 
	echo 2 > ${RDPMC_CTRL}
	echo 0 > ${BOOST_CTRL}
	echo 'performance' > ${GOV_CTRL}
	sysctl vm.mmap_min_addr=0
elif [[ ${1} == "off" ]]; then 
	echo 1 > ${RDPMC_CTRL}
	echo 1 > ${BOOST_CTRL}
	echo 'schedutil' > ${GOV_CTRL}
	sysctl vm.mmap_min_addr=65536
else
	echo "usage: $0 [off|on]"
	exit -1
fi
