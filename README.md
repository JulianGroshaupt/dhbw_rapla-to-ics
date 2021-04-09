# DHBW - rapla to ics

a tool to generate an ics file from rapla (timetable system at the DHBW Stuttgart)

## notice

This script was originally developed for the Rapla instance of the DHBW Stuttgart. It may work with other instances as well - feel free to test it, adapt it if necessary and contribute.

## todos

- [ ] extract locations from rapla

## deployment

### pull image

```
docker pull juliangroshaupt/dhbw_rapla-to-ics
```

### run container

```
docker run --rm \
           -e RAPLA_URL=<YOUR-RAPLA-URL-HERE> \
           -e RAPLA_START_YEAR=<YOUR-START-YEAR-HERE> \
           -e RAPLA_COURSE=<YOUR-COURSE-HERE> \
           -v /home/user/path/to/ics/web:/app/ics_files \
           juliangroshaupt/dhbw_rapla-to-ics
```

### environment variables

| key                         | description                                                                                                                                                 |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RAPLA_URL (required)        | the url to your rapla timetable (without parameters like _&day_, _&month_, _&year_ or _&today_)                                                             |
| RAPLA_START_YEAR (required) | which year to start parsing events from                                                                                                                     |
| RAPLA_COURSE                | the description of your course (if unset the script tries to get it from the timetable (h2 below the date selector) and will fail if this was not possible) |

### volumes

| path in container | description                                                               |
| ----------------- | ------------------------------------------------------------------------- |
| /app/ics_files    | the ics-file will be stored here (/app/ics_files/\<course\>/CALENDAR.ics) |

### automatic execution via cron

1. create a new file somewhere on your host machine with the following content

```
vi ~/rapla-to-ics_updater
```

```bash=
#!/bin/bash

# update image
docker pull juliangroshaupt/dhbw_rapla-to-ics

# <RAPLA_COURSE>
docker run --rm \
           -e RAPLA_URL=<RAPLA_URL> \
           -e RAPLA_START_YEAR=<RAPLA_START_YEAR> \
           -e RAPLA_COURSE=<RAPLA_COURSE> \
           -v /your/path/to/ics/www:/app/ics_files \
           juliangroshaupt/dhbw_rapla-to-ics
```

2. save and quit

3. make file executeable for your user and group only

```
chmod +x ~/rapla-to-ics_updater
chmod o-r ~/rapla-to-ics_updater
chmod o-x ~/rapla-to-ics_updater
```

4. edit crontab and add the following lines

```
crontab -e
```

```
# update rapla-ics (every 2 hours between 5 and 19h from monday through friday)
#  - see: https://crontab.guru/#0_5-19/2_*_*_1-5
0 5-19/2 * * 1-5 /home/<your_username>/rapla-to-ics_updater
```

5. save and quit

## license

Apache 2.0 License, see [LICENSE](https://github.com/JulianGroshaupt/dhbw_rapla-to-ics/blob/main/LICENSE)
