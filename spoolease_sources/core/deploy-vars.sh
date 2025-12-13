proj_dir=$(pwd)

path=$(pwd)
xtask_dir=""
while [ "$path" != "/" ]; do
    if [ -d "$path/esp-hal-app" ]; then
        xtask_dir="$path/esp-hal-app"
        break
    fi
    path=$(dirname "$path")
done
if [ -z "$xtask_dir" ]; then
    echo "${esp-hal-app} not found" >&2
    exit 1
fi

path=$(pwd)
base_target_dir=""
while [ "$path" != "/" ]; do
    if [ -d "$path/${base_target}" ]; then
        base_target_dir="$path/${base_target}"
        break
    fi
    path=$(dirname "$path")
done
if [ -z "$base_target_dir" ]; then
    echo "${base_target} not found" >&2
    exit 1
fi
