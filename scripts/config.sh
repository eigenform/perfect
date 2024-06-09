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
	./smt.sh off
	./rdpmc.sh on
	./freq.sh on
	./low-mmap.sh on
elif [[ ${1} == "off" ]]; then 
	./smt.sh on
	./rdpmc.sh off
	./freq.sh off
	./low-mmap.sh off
else
	echo "usage: $0 [off|on]"
	exit -1
fi
