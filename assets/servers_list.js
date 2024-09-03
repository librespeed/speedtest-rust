function get_servers() {
    // your server list here
    return [
        {
            name : "Your server name here",
            server : window.location.origin,
            dlURL: "backend/garbage", // edit this when you changed base_url in configs file
            ulURL: "backend/empty", // edit this when you changed base_url in configs file
            pingURL: "backend/empty", // edit this when you changed base_url in configs file
            getIpURL: "backend/getIP" // edit this when you changed base_url in configs file
        }
    ]
}