##
## Author: Pontus Laestadius
## Since: 2017-12-28
## 
## Will run a debugging version of the application using shell commands.
## Only run any following command if the previous one was successfull.

# Run the application.
cargo run && 

# Run the debug analysis.
python debug.py && 

# Remove the log after analysis.
rm log.txt

