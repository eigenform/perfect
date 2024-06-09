
SMT_CTL=/sys/devices/system/cpu/smt/control

if [[ -z ${1} ]]; then echo "usage: ${0} [off|on]"; exit -1; fi
if [[ $EUID != 0 ]]; then echo "must be root"; exit -1; fi

if [[ ${1} == "off" ]]; then
	echo 'off' > ${SMT_CTL}
elif [[ ${1} == "on" ]]; then
	echo 'on' > ${SMT_CTL}
else
	echo "usage: ${0} [off|on]"
	exit -1
fi


