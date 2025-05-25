openocd -f interface/stlink.cfg -f target/stm32f4x.cfg

:: netstat -aon|findstr "6666"
:: tasklist|findstr "4832" //PID