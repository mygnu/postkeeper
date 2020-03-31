#! /bin/bash
# System-V script to mange postkeeper daemon
# lives in `/etc/init.d/`


### BEGIN INIT INFO
# Provides:		postkeeper
# Short-Description: Manage PostKeeper service/daemon
# Description: PostKeep provides allow/block list milter service for postfix/sendmail

### END INIT INFO

PATH=/sbin:/bin:/usr/sbin:/usr/bin
DAEMON=/usr/sbin/postkeeper
NAME=postkeeper
DESC="PostKeeper"
RUNDIR=/var/run/postkeeper
USER=postkeeper
GROUP=postkeeper
PIDFILE=$RUNDIR/$NAME.pid
CONFIGFILE=/etc/postkeeper/postkeeper.ini

test -x $DAEMON || exit 0


# Include LSB provided init functions
. /lib/lsb/init-functions

pathfind() {
    OLDIFS="$IFS"
    IFS=:
    for p in $PATH; do
        if [ -x "$p/$*" ]; then
            IFS="$OLDIFS"
            return 0
        fi
    done
    IFS="$OLDIFS"
    return 1
}


get_pid() {
    cat "$PIDFILE"
}

is_running() {
    [ -f "$PIDFILE" ] && ps -p `get_pid` > /dev/null 2>&1
}

# run daemon with /etc/postkeeper.ini
DAEMON_OPTS="-c $CONFIGFILE -u $USER -g $GROUP"

start() {
    echo -n "Starting $DESC"
    if is_running; then
        echo "$DESC is already running"
        exit 0
    fi

    # test config and exit on fail
    $DAEMON $DAEMON_OPTS -t > /dev/null 2>&1
    if [[ $? -ne 0 ]] ; then
        exit 1
    fi

    # Create the run directory if it doesn't exist
    if [ ! -d "$RUNDIR" ]; then
        install -o "$USER" -g "$GROUP" -m 755 -d "$RUNDIR" || exit 2
        if pathfind restorecon; then restorecon "$RUNDIR"
                fi
    fi

    $DAEMON $DAEMON_OPTS > /dev/null 2>&1

    # try for maximum of 10 seconds for service to start
    for i in {1..20}
        do
            echo -n "."
            if is_running; then
                break
            fi

            sleep 0.5s
        done

    if ! is_running; then
        echo "Unable to start $DESC"
        exit 1
    fi
    echo "  Started"
}

stop() {
    if is_running; then
        echo -n "Stopping $DESC"
        kill `get_pid`

        # try for maximum of 10 seconds for service to stop completely
        for i in {1..20}
            do
                if ! is_running; then
                    break
                fi

                sleep 0.5s
                echo -n "."
            done

        if is_running; then
            echo "Not stopped; may still be shutting down or shutdown may have failed"
            exit 1
        else
            echo "  Stopped"
        fi
    else
        echo "$DESC Not running"
    fi
}

restart() {
    stop
    start
}

status() {
    if is_running; then
        echo "$DESC is Running"
    else
        echo "$DESC has Stopped"
        exit 1
    fi
}

case "$1" in
  start)
    start
    ;;
  stop)
    stop
    ;;
  restart|reload)
    restart
    ;;
  status)
    status
    ;;
  *)
    N=/etc/init.d/$NAME
    echo "Usage: $N {start|stop|restart|reload|status}" >&2
    exit 1
    ;;
esac

exit 0
