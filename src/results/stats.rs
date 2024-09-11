use std::sync::Arc;
use handlebars::Handlebars;
use serde_json::json;
use tokio::sync::Mutex;
use crate::config::SERVER_CONFIG;
use crate::database::Database;
use crate::http::cookie::{make_cookie, make_discard_cookie, validate_cookie};
use crate::http::request::Request;
use crate::http::response::Response;
use crate::results::TelemetryData;

pub async fn handle_stat_page (request : &Request,database : &mut Arc<Mutex<dyn Database + Send>>) -> Response {
    let server_config = SERVER_CONFIG.get().unwrap();
    let redirect_path = format!("{}/stats",server_config.base_url);
    // check database
    if server_config.database_type == "none" {
        return Response::res_200("Statistics are disabled")
    }
    // check stats password
    let no_password = server_config.stats_password.is_empty();
    let mut logged_in = false;
    let mut password_wrong = false;
    let mut telemetry_list : Vec<TelemetryData> = Vec::new();

    let mut db = database.lock().await;

    //check login
    if !no_password {

        let op = request.query_params.get("op");
        let cookie_data = request.headers.get("Cookie");

        if validate_cookie(cookie_data) {
            if op == Some(&"logout".to_string()) {
                let cookie_discard = make_discard_cookie(&redirect_path);
                return Response::res_temporary_redirect_cookie(&cookie_discard,&redirect_path)
            } else {
                logged_in = true;
                let def = "L100".to_string();
                let id = request.query_params.get("id").unwrap_or(&def).as_str();
                match id {
                    "L100" => {
                        let data = db.fetch_last_100();
                        match data {
                            Ok(mut data) => {
                                telemetry_list.append(&mut data);
                                drop(data);
                            }
                            Err(_) => {
                                return Response::res_500()
                            }
                        }
                    }
                    _ => {
                        let data = db.fetch_by_uuid(id);
                        match data {
                            Ok(data) => {
                                if let Some(data) = data {
                                    telemetry_list.push(data)
                                }
                            }
                            Err(_) => {
                                return Response::res_500()
                            }
                        }
                    }
                }
            }
        } else if op == Some(&"login".to_string()) {
            let input_pass = request.form_data.get("password");
            if input_pass == Some(&server_config.stats_password) {
                let cookie_data = make_cookie(&redirect_path);
                return Response::res_temporary_redirect_cookie(&cookie_data,&redirect_path)
            } else {
                password_wrong = true;
            }
        }
    }

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("stats_page",HTML_TEMPLATE).unwrap();
    let data = json!({
        "no_password": no_password,
        "logged_in": logged_in,
        "telemetry_list" : telemetry_list
    });

    let rendered_html = handlebars.render("stats_page",&data);
    match rendered_html {
        Ok(rendered_html) => {
            if password_wrong {
                Response::res_403_html(&rendered_html)
            } else {
                Response::res_200_html(&rendered_html)
            }
        }
        Err(_) => {
            Response::res_500()
        }
    }
}

const HTML_TEMPLATE : &str = r#"
<!DOCTYPE html>
<html>
<head>
<title>LibreSpeed - Stats</title>
<style type="text/css">
	html,body{
		margin:0;
		padding:0;
		border:none;
		width:100%; min-height:100%;
	}
	html{
		background-color: hsl(198,72%,35%);
		font-family: "Segoe UI","Roboto",sans-serif;
	}
	body{
		background-color:#FFFFFF;
		box-sizing:border-box;
		width:100%;
		max-width:70em;
		margin:4em auto;
		box-shadow:0 1em 6em #00000080;
		padding:1em 1em 4em 1em;
		border-radius:0.4em;
	}
	h1,h2,h3,h4,h5,h6{
		font-weight:300;
		margin-bottom: 0.1em;
	}
	h1{
		text-align:center;
	}
	table{
		margin:2em 0;
		width:100%;
	}
	table, tr, th, td {
		border: 1px solid #AAAAAA;
	}
	th {
		width: 6em;
	}
	td {
		word-break: break-all;
	}
</style>
</head>
<body>
<h1>LibreSpeed - Stats</h1>
{{#if no_password}}
		Please set stats_password in configs.toml to enable access.
{{else if logged_in}}
	<form action="stats" method="GET"><input type="hidden" name="op" value="logout" /><input type="submit" value="Logout" /></form>
	<form action="stats" method="GET">
		<h3>Search test results</h6>
		<input type="hidden" name="op" value="id" />
		<input type="text" name="id" id="id" placeholder="Test ID" value=""/>
		<input type="submit" value="Find" />
		<input type="submit" onclick="document.getElementById('id').value='L100'" value="Show last 100 tests" />
	</form>

    {{#each telemetry_list}}
	<table>
		<tr><th>Test ID</th><td>{{ this.uuid }}</td></tr>
		<tr><th>Date and time</th><td>{{ this.timestamp }}</td></tr>
		<tr><th>IP and ISP Info</th><td>{{ this.ip_address }}<br/>{{ this.isp_info }}</td></tr>
		<tr><th>User agent and locale</th><td>{{ this.user_agent }}<br/>{{ this.lang }}</td></tr>
		<tr><th>Download speed</th><td>{{ this.download }}</td></tr>
		<tr><th>Upload speed</th><td>{{ this.upload }}</td></tr>
		<tr><th>Ping</th><td>{{ this.ping }}</td></tr>
		<tr><th>Jitter</th><td>{{ this.jitter }}</td></tr>
		<tr><th>Log</th><td>{{ this.log }}</td></tr>
		<tr><th>Extra info</th><td>{{ this.extra }}</td></tr>
	</table>
	{{/each}}
{{else}}
	<form action="stats?op=login" method="POST">
		<h3>Login</h3>
		<input type="password" name="password" placeholder="Password" value=""/>
		<input type="submit" value="Login" />
	</form>
{{/if}}
</body>
</html>
"#;