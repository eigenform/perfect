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
	./scripts/smt.sh off
	./scripts/rdpmc.sh on
	./scripts/freq.sh on
	./scripts/low-mmap.sh on
elif [[ ${1} == "off" ]]; then 
	./scripts/smt.sh on
	./scripts/rdpmc.sh off
	./scripts/freq.sh off
	./scripts/low-mmap.sh off
else
	echo "usage: $0 [off|on]"
	exit -1
fi
