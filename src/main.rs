use dotenv::dotenv;
use ics::properties::{DtEnd, DtStart, Location, Organizer, Summary};
use ics::{escape_text, Event, ICalendar, Standard, TimeZone};
use log::{debug, info};
use scraper::{Html, Selector};
use std::env;
use std::fs;

#[allow(dead_code)]
struct DhbwEvent {
    name: String,
    date: String,
    start_time: String,
    end_time: String,
    location: String,
    lecturer: String,
    is_exam: bool,
}

#[tokio::main]
async fn main() {
    // setting up logging
    env_logger::init();

    // load env
    dotenv().ok();
    let rapla_url = env::var("RAPLA_URL").unwrap_or_default();
    let start_year: u32 = env::var("RAPLA_START_YEAR")
        .unwrap_or_default()
        .parse::<u32>()
        .unwrap_or_default();
    let mut course = env::var("RAPLA_COURSE").unwrap_or_default();

    // validate env values
    if rapla_url.is_empty() {
        println!("RAPLA_URL environment variable is required but unset");
        return;
    }
    if start_year == 0 {
        println!("RAPLA_START_YEAR environment variable is required but unset or invalid");
        return;
    }

    // log values
    info!("rapla_url is: {}", rapla_url);
    info!("rapla_start_year is: {}", start_year);
    if !course.is_empty() {
        info!("rapla_course is {}", course);
    }

    let mut last_week_number = String::new();
    let mut events: Vec<DhbwEvent> = Vec::new();

    // start reading events from rapla
    for y in start_year..(start_year + 3 + 1) {
        for m in 1..(12 + 1) {
            for d in (1..(28 + 1)).step_by(3) {
                // the current rapla url we are working on
                let url = format!("{}&day={}&month={}&year={}", rapla_url, d, m, y);
                debug!("current url is: {}", url);

                // get body & create document
                let res = reqwest::get(&url).await.unwrap();
                let body = res.text().await.unwrap();
                let document = Html::parse_document(&body.to_string());

                // get course if required
                if course.is_empty() {
                    if !last_week_number.is_empty() {
                        // getting the course wasn't successfull
                        println!("Unable to get the course from the timetable. Please set RAPLA_COURSE in your environment");
                        return;
                    } else {
                        let course_selector = Selector::parse("h2").unwrap();
                        course = document
                            .select(&course_selector)
                            .next()
                            .unwrap()
                            .inner_html()
                            .replace("\n", "");
                        debug!("extracted course is {}", course);
                    }
                }

                // only go on if we are in a new week
                let current_week_number_selector = Selector::parse("th.week_number").unwrap();
                let current_week_number = document
                    .select(&current_week_number_selector)
                    .next()
                    .unwrap()
                    .inner_html();
                if current_week_number == last_week_number {
                    continue;
                }
                // set last week to current week
                last_week_number = current_week_number.clone();

                // get week blocks
                let week_block_selector = Selector::parse("td.week_block").unwrap();
                for week_block in document.select(&week_block_selector) {
                    // get tooltip
                    let tooltip_selector = Selector::parse("span.tooltip").unwrap();
                    let tooltip = week_block.select(&tooltip_selector).next().unwrap();

                    // get event type (from tooltip)
                    let type_selector = Selector::parse("strong").unwrap();
                    let r#type = tooltip.select(&type_selector).next().unwrap().inner_html();

                    // stop if "Feiertag"
                    if r#type == "Feiertag" {
                        continue;
                    }

                    // get information table
                    let information_table_selector = Selector::parse("table.infotable").unwrap();
                    let information_table = week_block
                        .select(&information_table_selector)
                        .next()
                        .unwrap();

                    // get event name (from information table)
                    let event_name = information_table
                        .select(&Selector::parse("td").unwrap())
                        .nth(1)
                        .unwrap()
                        .inner_html();

                    // set event_is_exam (depending on event name or type)
                    let event_is_exam = event_name.starts_with("Klausur")
                        || event_name.starts_with("Nach-, Wiederholklausur")
                        || event_name.starts_with("PRAXIS II Prüfung")
                        || event_name.starts_with("Abschlusspräsentation")
                        || r#type == "Klausur / Prüfung";

                    // get string with event times
                    let event_times_string = week_block
                        .select(&Selector::parse("div").unwrap())
                        .nth(1)
                        .unwrap()
                        .inner_html();
                    let event_times_vec: Vec<_> = event_times_string.split(' ').collect();

                    // get weekday
                    let event_weekday = event_times_vec.get(0).unwrap().to_string();

                    // get date
                    // notice: this is not working for the "Klausuren"-placeholder-dates because they do not have the event_weekday part in their event_times_string
                    let mut event_date = String::new();
                    for week_header in
                        document.select(&Selector::parse("td.week_header nobr").unwrap())
                    {
                        if week_header.inner_html().starts_with(&event_weekday) {
                            event_date = format!(
                                "{}{}",
                                week_header
                                    .inner_html()
                                    .replace(&format!("{} ", event_weekday).to_string(), ""),
                                y
                            );
                            break;
                        }
                    }

                    // get times
                    let mut event_start_time = String::new();
                    let mut event_end_time = String::new();

                    // try to get event times from every "part" of the event_start_end_time_vec
                    for event_times in event_times_vec {
                        if event_times.contains('-') {
                            // get start and end times
                            let event_start_end_time_vec: Vec<_> = event_times.split('-').collect();

                            // set start and end times
                            event_start_time = event_start_end_time_vec.get(0).unwrap().to_string();
                            event_end_time = event_start_end_time_vec.get(1).unwrap().to_string();

                            break;
                        }
                    }

                    // exit if start- / end-time is empty
                    if event_start_time.is_empty() || event_end_time.is_empty() {
                        println!(
                            "Unable to get the start- / end-time for the event '{}' on {}. It looks like the entry in rapla does not match the expected format",
                            event_name, event_date
                        );
                        return;
                    }

                    // get lecturer
                    let mut event_lecturers_vec: Vec<String> = Vec::new();
                    let lecturers_selector = &Selector::parse("span.person").unwrap();
                    let lecturers = week_block.select(lecturers_selector);
                    for lecturer in lecturers {
                        if !event_is_exam {
                            let mut event_lecturer = lecturer.inner_html();

                            // remove trailing ,
                            if event_lecturer.ends_with(',') {
                                event_lecturer =
                                    event_lecturer[..(event_lecturer.len() - 1)].to_string();
                            }

                            // add lecturer to lecturers list
                            event_lecturers_vec.push(event_lecturer);
                        }
                    }
                    let event_lecturers = event_lecturers_vec.join(" & ");

                    // do not use event if some values are missing
                    if !(event_date.is_empty()) {
                        // log fetched values
                        info!(
                            "{}: event found: {}{} at {} from {} to {} with {}",
                            r#type,
                            event_name,
                            if event_is_exam { " (exam)" } else { "" },
                            event_date,
                            event_start_time,
                            event_end_time,
                            if !event_lecturers.is_empty() {
                                event_lecturers.clone()
                            } else {
                                "n/a".to_string()
                            },
                        );

                        // create event and push to events-vec
                        let event = DhbwEvent {
                            name: event_name,
                            date: event_date,
                            start_time: event_start_time,
                            end_time: event_end_time,
                            location: String::new(),
                            lecturer: event_lecturers,
                            is_exam: event_is_exam,
                        };
                        events.push(event);
                    }
                }
            }
        }
    }

    // start creating ics
    let mut calendar = ICalendar::new(
        "2.0",
        format!("-//DHBW Stuttgart//Stundenplan {}//DE", course),
    );
    calendar.add_timezone(TimeZone::standard(
        "Europe/Berlin",
        Standard::new("18930401T000632", "+0053", "+0100"),
    ));

    for event in events {
        // construct event id
        let id = format!(
            "{}_{}_{}-{}@course-{}.dhbw-stuttgart.de",
            event.name.replace(" ", "-").replace(",", ""),
            event.date.replace(".", "-"),
            event.start_time.replace(":", ""),
            event.end_time.replace(":", ""),
            course
        );

        // date formatted
        let mut date_vec: Vec<_> = event.date.split('.').collect();
        date_vec.reverse();
        let date = date_vec.join("");

        // start time formatted
        let start_time = format!("{}00", event.start_time.replace(":", ""));

        // end time formatted
        let end_time = format!("{}00", event.end_time.replace(":", ""));

        // create ics event
        let mut ics_event = Event::new(id, format!("{}T{}", date, start_time));
        ics_event.push(Summary::new(escape_text(event.name)));
        ics_event.push(DtStart::new(format!("{}T{}", date, start_time)));
        ics_event.push(DtEnd::new(format!("{}T{}", date, end_time)));
        ics_event.push(Organizer::new(escape_text(event.lecturer)));
        ics_event.push(Location::new(escape_text(event.location)));

        // add event to calendar
        calendar.add_event(ics_event);
    }

    // save calendar file
    let directory_path = format!("ics_files/{}", course);
    let file_name = "CALENDAR.ics";
    let full_path = format!("{}/{}", directory_path, file_name);
    fs::create_dir_all(directory_path).unwrap();
    calendar.save_file(full_path.clone()).unwrap();
    info!("saved file at: {}", full_path);

    // print finish
    println!("job for course {} finished", course);
}
