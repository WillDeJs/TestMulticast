# UDP Multicast Testing Tool
This is a very simple tool created with the sole purpose of easily testing multicast being enabled in a network.

This tool allows you to join a multicast group having its address and port. After the group is joined it will show you messages captured on it. 
You will also have the ability to send simple test messages to ensure your machine is allowed both join receive and send multicast messages.

![Start-up Screenshot](/assets/mctest_one.png)

## Limitations
This tool is very simple thus it's very limited on the features it provides. It does not replace Wireshark. It does not offer you the ability to capture all packets in the network. It is limited to capturing UDP Multicast packets only. It will not allow you to save a capture or run it for days uninterrupted. 

The main purpose of this tool is to test the ability to receive and send multicast messages on a given machine. This tool was simply born from the need of said tool while troubleshooting on site. It is meant to be a lightweight replacement for VLC stream testing.

Note: The tool is not meant to run forever. After 1000 messages are received in a session, the multicast group registration is terminated. This hard-coded limit is meant to avoid the tool using up too much resources. It is not expected that the tool would need tor un for days at at time.

## Usage

### Join Multicast Group
To use the tool simply:
1. Launch the tool.
2. Enter a Multicast Address and Port (optionally provide  TTL value).
3. Click the register button.
4. You are now registered.

![Capture Screenshot](/assets/mctest_two.png)

### Send Data to Multicast Group
Once connected you can send test messages (limited to UTF-8 strings) to the group.
1. Type a message on the   `Test Data` box.
2. Click the `Send` button.

### Analyzing Data
Once data is collected you can click on each captured message to quickly get it decoded a an ASCII message.

Optionally you can type filtering test in the search box. Press enter to search. Note that what you enter will be searched in all columns in the table (UTF-8 conversion is applied to data). This only provides a simple form of filtering. Only packets found to contain the entered string in the search box, will be displayed in the table.


![Capture Screenshot](/assets/mctest_three.png)