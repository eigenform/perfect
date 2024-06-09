#!/bin/bash

if [[ -z $1 ]]; then echo "usage: $0 [on|off]"; exit -1; fi
if [[ $EUID != 0 ]]; then echo "Must be root"; exit -1; fi

BOOST_CTRL=/sys/devices/system/cpu/cpufreq/boost
GOV_CTRL=/sys/devices/system/cpu/cpufreq/policy15/scaling_governor


if [[ ${1} == "on" ]]; then 
	echo 0 > ${BOOST_CTRL}
	echo 'performance' > ${GOV_CTRL}
elif [[ ${1} == "off" ]]; then 
	echo 1 > ${BOOST_CTRL}
	echo 'schedutil' > ${GOV_CTRL}
else
	echo "usage: $0 [off|on]"
	exit -1
fi
