const fs = require("fs").promises;
const path = require("path");
const process = require("process");
const { authenticate } = require("@google-cloud/local-auth");
const { google } = require("googleapis");
// const { exec } = require("child_process");

var assignments = [];

async function getSecondArg() {
    console.log("Reading assignments...");
    const ret = process.argv[2];
    console.log(ret);
    assignments = [JSON.parse(ret)];
    return ret;
}

// If modifying these scopes, delete token.json.
const SCOPES = ["https://www.googleapis.com/auth/calendar.events"];
// The file token.json stores the user's access and refresh tokens, and is
// created automatically when the authorization flow completes for the first
// time.
const TOKEN_PATH = path.join(process.cwd(), "token.json");
const CREDENTIALS_PATH = path.join(process.cwd(), "credentials.json");

/**
 * Reads previously authorized credentials from the save file.
 *
 * @return {Promise<OAuth2Client|null>}
 */
async function loadSavedCredentialsIfExist() {
    try {
        const content = await fs.readFile(TOKEN_PATH);
        const credentials = JSON.parse(content);
        return google.auth.fromJSON(credentials);
    } catch (err) {
        return null;
    }
}

/**
 * Serializes credentials to a file compatible with GoogleAUth.fromJSON.
 *
 * @param {OAuth2Client} client
 * @return {Promise<void>}
 */
async function saveCredentials(client) {
    const content = await fs.readFile(CREDENTIALS_PATH);
    const keys = JSON.parse(content);
    const key = keys.installed || keys.web;
    const payload = JSON.stringify({
        type: "authorized_user",
        client_id: key.client_id,
        client_secret: key.client_secret,
        refresh_token: client.credentials.refresh_token,
    });
    await fs.writeFile(TOKEN_PATH, payload);
}

/**
 * Load or request or authorization to call APIs.
 *
 */
async function authorize() {
    let client = await loadSavedCredentialsIfExist();
    if (client) {
        return client;
    }
    client = await authenticate({
        scopes: SCOPES,
        keyfilePath: CREDENTIALS_PATH,
    });
    if (client.credentials) {
        await saveCredentials(client);
    }
    return client;
}

async function addEvent(auth) {
    const calendar = google.calendar({ version: "v3", auth });
    const res = await calendar.events.list({
        calendarId: "primary",
        timeMin: new Date().toISOString(),
        maxResults: 10000,
        singleEvents: true,
        auth: auth,
        orderBy: "startTime",
    });
    var events = res.data.items;
    // events = events.map((event, _) => {
    //     return event.summary;
    // });
    for (var i = 0; i < assignments.length; i++) {
        if (events.some((event) => event.summary === assignments[i].name)) {
            if (assignments[i].done) {
                console.log("Event already exists, deleting...");
                const prom = new Promise((resolve, _) => {
                    calendar.events.delete(
                        {
                            calendarId: "primary",
                            eventId: events.find(
                                (event) =>
                                    event.summary === assignments[i].name,
                            ).id,
                        },
                        function (_, event) {
                            resolve();
                            console.log("Event deleted: ", event.data.summary);
                        },
                    );
                });
                await prom;
            } else if (
                events.some((event) => event.summary === assignments[i].name) &&
                !events.some(
                    (event) =>
                        Date.parse(event.start?.dateTime) ===
                        assignments[i].due * 1000,
                )
            ) {
                console.log("Event already exists, modifying...");
                const newEvent = {
                    summary: assignments[i].name,
                    location: "Anywhere",
                    description: `Due for ${assignments[i].course}`,
                    start: {
                        dateTime: new Date(
                            assignments[i].due * 1000,
                        ).toISOString(),
                        timeZone: "America/New_York",
                    },
                    end: {
                        dateTime: new Date(
                            assignments[i].due * 1000,
                        ).toISOString(),
                        timeZone: "America/New_York",
                    },
                };
                const prom = new Promise((resolve, _) => {
                    calendar.events.update(
                        {
                            calendarId: "primary",
                            eventId: events.find(
                                (event) =>
                                    event.summary === assignments[i].name,
                            ).id,
                            resource: newEvent,
                        },
                        function (_, event) {
                            resolve();
                            console.log("Event updated: ", event.data.summary);
                        },
                    );
                });
                await prom;
            }
            continue;
        }
        const event = {
            summary: assignments[i].name,
            location: "Anywhere",
            description: `Due for ${assignments[i].course}`,
            start: {
                dateTime: new Date(assignments[i].due * 1000).toISOString(),
                timeZone: "America/New_York",
            },
            end: {
                dateTime: new Date(assignments[i].due * 1000).toISOString(),
                timeZone: "America/New_York",
            },
        };
        console.log("Adding event: ", event.summary);
        const prom = new Promise((resolve, _) => {
            // console.log(event);
            calendar.events.insert(
                {
                    api_key: process.env.GOOGLE_API_KEY,
                    auth: auth,
                    calendarId: "primary",
                    resource: event,
                },
                function (_, event) {
                    console.log("Event created: ", event.data.summary);
                    resolve();
                },
            );
        });
        await prom;
    }
    return auth;
}

getSecondArg()
    .then(authorize)
    .then(addEvent)
    // .then(listEvents)
    .catch(console.log);
