#!/usr/bin/env python3
"""
Selenium-based YouTube extractor for bypassing anti-bot measures
This script uses a real browser to extract video information and download URLs
"""

import json
import sys
import time
import random
import argparse
import tempfile
import subprocess
import os
import re
import asyncio
import threading
from typing import Optional

try:
    from selenium import webdriver
    from selenium.webdriver.chrome.options import Options
    from selenium.webdriver.common.by import By
    from selenium.webdriver.support.ui import WebDriverWait
    from selenium.webdriver.support import expected_conditions as EC
    from selenium.webdriver.common.action_chains import ActionChains
    from selenium.common.exceptions import (
        TimeoutException, 
        NoSuchElementException, 
        WebDriverException,
        SessionNotCreatedException
    )
    from anti_detection import anti_detection
except ImportError as e:
    print(f"Error: Required packages not installed. Install with: pip install selenium", file=sys.stderr)
    sys.exit(1)

class YouTubeSeleniumExtractor:
    def __init__(self, headless=True, use_anti_detection=True):
        self.driver = None
        self.headless = headless
        self.session_id = random.randint(100000, 999999)
        self.anti_detection = None
        self.rotation_thread = None
        self.rotation_active = False
        
        # Initialize anti-detection system
        if use_anti_detection:
            try:
                import anti_detection
                self.anti_detection = anti_detection.AdvancedAntiDetection()
                print("[+] Anti-detection system initialized")
            except ImportError:
                print("[-] Anti-detection module not available, proceeding without it")
                self.anti_detection = None
        
        self.setup_driver()
        
        # Start continuous rotation service
        if self.anti_detection:
            self.start_rotation_service()
    
    async def setup_driver(self):
        """Setup Chrome driver with advanced anti-detection measures"""
        # Check if we should rotate proxy
        if anti_detection.should_rotate_proxy():
            await anti_detection.rotate_proxy()
        
        # Get base Chrome options with evasion
        chrome_options = anti_detection.get_chrome_options_with_evasion()
        
        if self.headless:
            chrome_options.add_argument('--headless=new')
        
        # Get random user agent
        user_agent = anti_detection.get_random_user_agent()
        chrome_options.add_argument(f'--user-agent={user_agent}')
        
        # Container-specific arguments (essential for Docker/hosting)
        container_args = [
            '--no-sandbox',
            '--disable-dev-shm-usage',
            '--disable-gpu',
            '--disable-software-rasterizer',
            '--disable-features=VizDisplayCompositor',
            '--disable-ipc-flooding-protection',
            '--memory-pressure-off',
            '--max_old_space_size=4096',
            '--aggressive-cache-discard',
            '--disable-background-networking',
            '--ignore-certificate-errors',
            '--allow-running-insecure-content',
            '--disable-web-security',
            '--disable-features=SafeBrowsing',
        ]
        
        for arg in container_args:
            chrome_options.add_argument(arg)
        
        # Add proxy if available
        proxy_dict = anti_detection.get_proxy_dict()
        if proxy_dict:
            proxy_host = anti_detection.current_proxy.host
            proxy_port = anti_detection.current_proxy.port
            chrome_options.add_argument(f'--proxy-server=http://{proxy_host}:{proxy_port}')
        
        # Set Chrome binary path for containers
        chrome_binary_paths = [
            '/usr/bin/chromium',
            '/usr/bin/chromium-browser', 
            '/usr/bin/google-chrome',
            '/usr/bin/google-chrome-stable'
        ]
        
        for binary_path in chrome_binary_paths:
            if os.path.exists(binary_path):
                chrome_options.binary_location = binary_path
                break

        try:
            self.driver = webdriver.Chrome(options=chrome_options)
            
            # Inject evasion scripts
            anti_detection.inject_evasion_scripts(self.driver)
            
            # Set window size to random realistic resolution
            resolutions = [(1920, 1080), (1366, 768), (1440, 900), (1536, 864), (1280, 720)]
            width, height = random.choice(resolutions)
            self.driver.set_window_size(width, height)
            
            print(f"Session {self.session_id}: Browser initialized with {user_agent[:50]}...")
            
        except SessionNotCreatedException as e:
            print(f"Failed to create Chrome session: {e}", file=sys.stderr)
            sys.exit(1)
        except WebDriverException as e:
            print(f"Chrome WebDriver error: {e}", file=sys.stderr)
            sys.exit(1)
        except Exception as e:
            print(f"Unexpected error setting up Chrome driver: {e}", file=sys.stderr)
            sys.exit(1)
    
    def setup_driver(self, headless=True):
        """Setup Chrome driver with anti-detection measures and container compatibility"""
        chrome_options = Options()
        
        if headless:
            chrome_options.add_argument('--headless=new')
        
        # Container-specific arguments (essential for Docker/hosting)
        chrome_options.add_argument('--no-sandbox')
        chrome_options.add_argument('--disable-dev-shm-usage')
        chrome_options.add_argument('--disable-gpu')
        chrome_options.add_argument('--disable-software-rasterizer')
        chrome_options.add_argument('--disable-background-timer-throttling')
        chrome_options.add_argument('--disable-backgrounding-occluded-windows')
        chrome_options.add_argument('--disable-renderer-backgrounding')
        chrome_options.add_argument('--disable-features=TranslateUI')
        chrome_options.add_argument('--disable-ipc-flooding-protection')
        chrome_options.add_argument('--disable-features=VizDisplayCompositor')
        
        # Anti-detection arguments
        chrome_options.add_argument('--disable-blink-features=AutomationControlled')
        chrome_options.add_experimental_option("excludeSwitches", ["enable-automation"])
        chrome_options.add_experimental_option('useAutomationExtension', False)
        
        # Realistic browser settings (JavaScript MUST be enabled for YouTube)
        chrome_options.add_argument('--user-agent=Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36')
        chrome_options.add_argument('--window-size=1920,1080')
        chrome_options.add_argument('--disable-extensions')
        chrome_options.add_argument('--disable-plugins')
        chrome_options.add_argument('--disable-images')  # Keep images disabled for performance
        chrome_options.add_argument('--disable-javascript-harmony-shipping')
        chrome_options.add_argument('--disable-web-security')  # Sometimes needed for CORS
        chrome_options.add_argument('--ignore-certificate-errors')
        chrome_options.add_argument('--allow-running-insecure-content')
        chrome_options.add_argument('--disable-web-security')
        chrome_options.add_argument('--disable-features=SafeBrowsing')
        
        # Memory and performance optimizations for hosting
        chrome_options.add_argument('--memory-pressure-off')
        chrome_options.add_argument('--max_old_space_size=4096')
        chrome_options.add_argument('--aggressive-cache-discard')
        chrome_options.add_argument('--disable-background-networking')
        
        # Set Chrome binary path for containers
        chrome_binary_paths = [
            '/usr/bin/chromium',
            '/usr/bin/chromium-browser', 
            '/usr/bin/google-chrome',
            '/usr/bin/google-chrome-stable'
        ]
        
        for binary_path in chrome_binary_paths:
            if os.path.exists(binary_path):
                chrome_options.binary_location = binary_path
                break

        try:
            self.driver = webdriver.Chrome(options=chrome_options)
            # Execute script to remove webdriver property
            self.driver.execute_script("Object.defineProperty(navigator, 'webdriver', {get: () => undefined})")
        except SessionNotCreatedException as e:
            print(f"Failed to create Chrome session: {e}", file=sys.stderr)
            sys.exit(1)
        except WebDriverException as e:
            print(f"Chrome WebDriver error: {e}", file=sys.stderr)
            sys.exit(1)
        except Exception as e:
            print(f"Unexpected error setting up Chrome driver: {e}", file=sys.stderr)
            sys.exit(1)
    
    def human_like_delay(self, min_delay=1, max_delay=3):
        """Add advanced human-like delays with realistic patterns"""
        delay = anti_detection.get_random_delay(min_delay, max_delay)
        
        # Simulate different types of delays
        delay_types = [
            'reading',     # Longer, consistent delay
            'thinking',    # Medium delay with pauses
            'scanning',    # Quick, jittery movements
            'typing'       # Variable delay
        ]
        
        delay_type = random.choice(delay_types)
        
        if delay_type == 'reading':
            # Simulate reading - longer, steady delay
            time.sleep(delay)
        elif delay_type == 'thinking':
            # Simulate thinking - delay with small pauses
            total_delay = delay
            while total_delay > 0:
                pause = min(random.uniform(0.1, 0.5), total_delay)
                time.sleep(pause)
                total_delay -= pause
        elif delay_type == 'scanning':
            # Simulate quick scanning - multiple short delays
            num_pauses = random.randint(2, 5)
            for _ in range(num_pauses):
                time.sleep(delay / num_pauses + random.uniform(-0.1, 0.1))
        else:  # typing
            # Simulate typing rhythm
            time.sleep(delay * random.uniform(0.8, 1.2))

    async def advanced_bot_detection_evasion(self, driver, url):
        """Advanced evasion techniques with continuous rotation"""
        if not self.anti_detection:
            return
        
        try:
            # Inject advanced evasion scripts
            evasion_scripts = [
                "Object.defineProperty(navigator, 'webdriver', {get: () => undefined});",
                "Object.defineProperty(navigator, 'plugins', {get: () => [1, 2, 3, 4, 5]});",
                "Object.defineProperty(navigator, 'languages', {get: () => ['en-US', 'en']});",
                "window.chrome = {runtime: {}};",
                "Object.defineProperty(navigator, 'permissions', {get: () => ({query: x => Promise.resolve({state: 'granted'})})});",
                "Object.defineProperty(navigator, 'deviceMemory', {get: () => 8});",
                "Object.defineProperty(navigator, 'hardwareConcurrency', {get: () => 4});",
                "Object.defineProperty(screen, 'colorDepth', {get: () => 24});",
                "Object.defineProperty(navigator, 'connection', {get: () => ({effectiveType: '4g', downlink: 10})});",
                """
                // Advanced fingerprint spoofing
                const originalCanvas = HTMLCanvasElement.prototype.getContext;
                HTMLCanvasElement.prototype.getContext = function(type) {
                    if (type === '2d' || type === 'webgl') {
                        const context = originalCanvas.call(this, type);
                        const originalGetImageData = context.getImageData;
                        context.getImageData = function() {
                            const imageData = originalGetImageData.apply(this, arguments);
                            for (let i = 0; i < imageData.data.length; i += 4) {
                                imageData.data[i] += Math.floor(Math.random() * 10) - 5;
                                imageData.data[i + 1] += Math.floor(Math.random() * 10) - 5;
                                imageData.data[i + 2] += Math.floor(Math.random() * 10) - 5;
                            }
                            return imageData;
                        };
                        return context;
                    }
                    return originalCanvas.call(this, type);
                };
                """,
                """
                // Spoof WebRTC
                window.RTCPeerConnection = window.RTCPeerConnection || window.webkitRTCPeerConnection || window.mozRTCPeerConnection;
                if (window.RTCPeerConnection) {
                    const originalCreateDataChannel = window.RTCPeerConnection.prototype.createDataChannel;
                    window.RTCPeerConnection.prototype.createDataChannel = function() {
                        return originalCreateDataChannel.apply(this, arguments);
                    };
                }
                """
            ]
            
            for script in evasion_scripts:
                try:
                    driver.execute_script(script)
                except Exception as e:
                    print(f"Failed to execute evasion script: {e}")
            
            # Advanced behavioral simulation
            await self.simulate_realistic_browsing(driver)
            
            print("[+] Advanced bot detection evasion activated")
            
        except Exception as e:
            print(f"[-] Advanced evasion failed: {e}")

    async def simulate_realistic_browsing(self, driver):
        """Simulate realistic human browsing behavior"""
        try:
            # Random scroll patterns
            for _ in range(random.randint(2, 5)):
                scroll_amount = random.randint(100, 800)
                direction = random.choice(['down', 'up'])
                
                if direction == 'down':
                    driver.execute_script(f"window.scrollBy(0, {scroll_amount});")
                else:
                    driver.execute_script(f"window.scrollBy(0, -{scroll_amount});")
                
                await asyncio.sleep(random.uniform(0.5, 2.0))
            
            # Random mouse movements
            try:
                from selenium.webdriver.common.action_chains import ActionChains
                actions = ActionChains(driver)
                
                for _ in range(random.randint(3, 8)):
                    x = random.randint(50, 800)
                    y = random.randint(50, 600)
                    actions.move_by_offset(x, y)
                    actions.pause(random.uniform(0.1, 0.5))
                
                actions.perform()
            except Exception as e:
                print(f"Mouse simulation failed: {e}")
            
            # Random page interactions
            try:
                clickable_elements = driver.find_elements(By.CSS_SELECTOR, "button, a, .clickable")
                if clickable_elements and random.random() < 0.3:  # 30% chance to click something
                    element = random.choice(clickable_elements[:5])  # Only first 5 to avoid unwanted navigation
                    if element.is_displayed() and element.is_enabled():
                        driver.execute_script("arguments[0].click();", element)
                        await asyncio.sleep(random.uniform(0.5, 1.5))
            except Exception as e:
                print(f"Element interaction failed: {e}")
                
        except Exception as e:
            print(f"[-] Realistic browsing simulation failed: {e}")

    async def rotate_session_every_few_seconds(self, driver):
        """Continuously rotate detection parameters"""
        if not self.anti_detection:
            return
            
        try:
            # Rotate user agent
            new_ua = self.anti_detection.get_random_user_agent()
            driver.execute_script(f"Object.defineProperty(navigator, 'userAgent', {{get: () => '{new_ua}'}});")
            
            # Rotate viewport
            width = random.randint(1200, 1920)
            height = random.randint(800, 1080)
            driver.set_window_size(width, height)
            
            # Rotate timezone
            timezones = ['America/New_York', 'Europe/London', 'Asia/Tokyo', 'Australia/Sydney', 'America/Los_Angeles']
            timezone = random.choice(timezones)
            driver.execute_script(f"""
                Intl.DateTimeFormat = class extends Intl.DateTimeFormat {{
                    constructor(...args) {{
                        super(...args);
                        this.resolvedOptions = () => ({{
                            ...super.resolvedOptions(),
                            timeZone: '{timezone}'
                        }});
                    }}
                }};
            """)
            
            # Rotate language preferences
            languages = [
                ['en-US', 'en'],
                ['en-GB', 'en'],
                ['en-CA', 'en'],
                ['en-AU', 'en']
            ]
            lang = random.choice(languages)
            driver.execute_script(f"Object.defineProperty(navigator, 'languages', {{get: () => {lang}}});")
            
            print("[+] Session parameters rotated")
            
        except Exception as e:
            print(f"[-] Session rotation failed: {e}")

    def start_rotation_service(self):
        """Start background thread for continuous parameter rotation"""
        if not self.anti_detection or self.rotation_active:
            return
            
        self.rotation_active = True
        self.rotation_thread = threading.Thread(target=self._rotation_worker, daemon=True)
        self.rotation_thread.start()
        print("[+] Continuous rotation service started")

    def stop_rotation_service(self):
        """Stop the background rotation service"""
        self.rotation_active = False
        if self.rotation_thread:
            self.rotation_thread.join(timeout=5)
        print("[+] Rotation service stopped")

    def _rotation_worker(self):
        """Background worker for continuous rotation"""
        while self.rotation_active and self.driver:
            try:
                # Wait between 3-8 seconds before rotating
                rotation_interval = random.uniform(3, 8)
                time.sleep(rotation_interval)
                
                if not self.rotation_active or not self.driver:
                    break
                
                # Run rotation in async context
                try:
                    loop = asyncio.new_event_loop()
                    asyncio.set_event_loop(loop)
                    loop.run_until_complete(self.rotate_session_every_few_seconds(self.driver))
                    loop.close()
                except Exception as e:
                    print(f"Background rotation failed: {e}")
                    
            except Exception as e:
                print(f"Rotation worker error: {e}")
                time.sleep(5)  # Wait before retrying
    
    def extract_video_id(self, url):
        """Extract video ID from YouTube URL"""
        if not url:
            return None
            
        patterns = [
            r'(?:youtube\.com/watch\?v=|youtu\.be/|youtube\.com/embed/)([a-zA-Z0-9_-]{11})',
            r'youtube\.com/v/([a-zA-Z0-9_-]{11})',
        ]
        
        for pattern in patterns:
            match = re.search(pattern, url)
            if match:
                return match.group(1)
        return None
    
    async def get_video_info_async(self, url):
        """Async wrapper for get_video_info with advanced evasion"""
        if not self.driver:
            return {"error": "WebDriver not initialized"}
            
        try:
            video_id = self.extract_video_id(url)
            if not video_id:
                return {"error": "Invalid YouTube URL"}

            # Apply advanced anti-detection before navigation
            await self.advanced_bot_detection_evasion(self.driver, url)
            
            # Navigate to the video page
            self.driver.get(f"https://www.youtube.com/watch?v={video_id}")
            
            # Rotate session parameters every few seconds
            await self.rotate_session_every_few_seconds(self.driver)
            
            self.human_like_delay(3, 6)  # Longer delay for better loading
            
            # Continue with regular extraction...
            return self.get_video_info(url)
            
        except Exception as e:
            print(f"[-] Async video info extraction failed: {e}")
            return {"error": f"Failed to extract video information: {str(e)}"}

    def get_video_info(self, url):
        """Extract video information using Selenium"""
        if not self.driver:
            return {"error": "WebDriver not initialized"}
            
        try:
            video_id = self.extract_video_id(url)
            if not video_id:
                return {"error": "Invalid YouTube URL"}
            
            # For synchronous calls, run async evasion in background
            try:
                import asyncio
                loop = asyncio.new_event_loop()
                asyncio.set_event_loop(loop)
                loop.run_until_complete(self.advanced_bot_detection_evasion(self.driver, url))
                loop.close()
            except Exception as e:
                print(f"Background evasion failed: {e}")
            
            # Navigate to the video page
            self.driver.get(f"https://www.youtube.com/watch?v={video_id}")
            self.human_like_delay(3, 6)  # Longer delay for better loading
            
            # Wait for page to load and try to click play to trigger full loading
            wait = WebDriverWait(self.driver, 20)  # Increased timeout
            
            # Try to ensure the page is fully loaded
            try:
                # Wait for basic page structure
                wait.until(EC.presence_of_element_located((By.TAG_NAME, "body")))
                self.human_like_delay(2, 3)
                
                # Scroll down a bit to trigger lazy loading
                self.driver.execute_script("window.scrollTo(0, 300);")
                self.human_like_delay(1, 2)
            except TimeoutException:
                pass  # Continue anyway
            
            # Extract video title with multiple fallback selectors and better waiting
            title = "Unknown Title"
            title_selectors = [
                "h1.ytd-watch-metadata yt-formatted-string",
                "h1.ytd-video-primary-info-renderer",
                "h1.style-scope.ytd-video-primary-info-renderer", 
                "h1[class*='title']",
                ".ytd-video-primary-info-renderer h1",
                "#container h1",
                "meta[property='og:title']",  # Fallback to meta tag
                "title"  # Last resort - page title
            ]
            
            for selector in title_selectors:
                try:
                    if selector == "meta[property='og:title']":
                        # Get title from meta tag
                        title_element = self.driver.find_element(By.CSS_SELECTOR, selector)
                        title = title_element.get_attribute("content").strip()
                    elif selector == "title":
                        # Get from page title and clean it
                        page_title = self.driver.title
                        title = page_title.replace(" - YouTube", "").strip()
                    else:
                        # Regular element text
                        title_element = wait.until(
                            EC.presence_of_element_located((By.CSS_SELECTOR, selector))
                        )
                        title = title_element.text.strip()
                    
                    if title and title != "Unknown Title" and len(title) > 0:
                        break
                except (TimeoutException, NoSuchElementException):
                    continue
            
            # Extract channel name with multiple selectors
            channel = "Unknown Channel"
            channel_selectors = [
                "#channel-name a",
                ".ytd-channel-name a",
                "#owner-text a",
                ".ytd-video-owner-renderer a",
                "[class*='channel'] a"
            ]
            
            for selector in channel_selectors:
                try:
                    channel_element = self.driver.find_element(By.CSS_SELECTOR, selector)
                    channel = channel_element.text.strip()
                    if channel and channel != "Unknown Channel":
                        break
                except NoSuchElementException:
                    continue
            
            # Extract duration with multiple selectors
            duration = None
            duration_selectors = [
                ".ytp-time-duration",
                ".ytd-thumbnail-overlay-time-status-renderer",
                "[class*='duration']",
                ".badge-style-type-simple"
            ]
            
            for selector in duration_selectors:
                try:
                    duration_element = self.driver.find_element(By.CSS_SELECTOR, selector)
                    duration = duration_element.text.strip()
                    if duration:
                        break
                except NoSuchElementException:
                    continue
            
            # Extract view count with multiple selectors and better parsing
            view_count = None
            view_selectors = [
                "#info #count .view-count",
                ".ytd-video-view-count-renderer",
                "[class*='view-count']",
                "#count .style-scope"
            ]
            
            for selector in view_selectors:
                try:
                    views_element = self.driver.find_element(By.CSS_SELECTOR, selector)
                    views_text = views_element.text.strip()
                    # Extract numbers from view count (handle K, M, B suffixes)
                    views_match = re.search(r'([\d,]+(?:\.\d+)?)\s*([KMB]?)', views_text.replace(',', ''))
                    if views_match:
                        number_str = views_match.group(1)
                        suffix = views_match.group(2).upper() if views_match.group(2) else ''
                        try:
                            number = float(number_str)
                            multiplier = {'K': 1000, 'M': 1000000, 'B': 1000000000}.get(suffix, 1)
                            view_count = int(number * multiplier)
                            break
                        except ValueError:
                            continue
                except (NoSuchElementException, ValueError, AttributeError):
                    continue
            
            # Extract thumbnail
            thumbnail = f"https://img.youtube.com/vi/{video_id}/maxresdefault.jpg"
            
            return {
                "id": video_id,
                "title": title,
                "channel": channel,
                "duration": duration,
                "view_count": view_count,
                "thumbnail": thumbnail,
                "url": url
            }
            
        except WebDriverException as e:
            return {"error": f"WebDriver error: {str(e)}"}
        except TimeoutException as e:
            return {"error": f"Page load timeout: {str(e)}"}
        except Exception as e:
            return {"error": f"Failed to extract video info: {str(e)}"}
    
    async def download_video_async(self, url, output_path, format_type="mp3", quality="192"):
        """Async download with advanced evasion"""
        if not self.driver:
            return {"error": "WebDriver not initialized"}
            
        try:
            # Apply advanced anti-detection before download
            await self.advanced_bot_detection_evasion(self.driver, url)
            
            # Get video info with evasion
            video_info = await self.get_video_info_async(url)
            if "error" in video_info:
                return video_info
            
            # Rotate session during download process
            await self.rotate_session_every_few_seconds(self.driver)
            
            # Continue with regular download process
            return self.download_video(url, output_path, format_type, quality)
            
        except Exception as e:
            print(f"[-] Async download failed: {e}")
            return {"error": f"Download failed: {str(e)}"}

    def download_video(self, url, output_path, format_type="mp3", quality="192"):
        """Download video using yt-dlp with cookies from browser session"""
        if not self.driver:
            return {"error": "WebDriver not initialized"}
            
        try:
            # Apply background evasion for synchronous calls
            try:
                import asyncio
                loop = asyncio.new_event_loop()
                asyncio.set_event_loop(loop)
                loop.run_until_complete(self.advanced_bot_detection_evasion(self.driver, url))
                loop.close()
            except Exception as e:
                print(f"Background evasion in download failed: {e}")
            
            video_info = self.get_video_info(url)
            if "error" in video_info:
                return video_info
            
            # Get cookies from browser session
            cookies = self.driver.get_cookies()
            
            # Save cookies to a temporary file in Netscape format
            cookie_file = None
            try:
                with tempfile.NamedTemporaryFile(mode='w', suffix='.txt', delete=False) as f:
                    f.write("# Netscape HTTP Cookie File\n")
                    for cookie in cookies:
                        domain = cookie.get('domain', '')
                        path = cookie.get('path', '/')
                        secure = 'TRUE' if cookie.get('secure', False) else 'FALSE'
                        name = cookie.get('name', '')
                        value = cookie.get('value', '')
                        
                        f.write(f"{domain}\tTRUE\t{path}\t{secure}\t0\t{name}\t{value}\n")
                    cookie_file = f.name
            except OSError as e:
                return {"error": f"Failed to create cookie file: {str(e)}"}
            
            # Prepare yt-dlp command with browser cookies
            if format_type == "mp3":
                cmd = [
                    "yt-dlp",
                    "--cookies", cookie_file,
                    "--extract-flat", "false",
                    "--format", "bestaudio",
                    "--audio-format", "mp3",
                    "--audio-quality", quality,
                    "--embed-metadata",
                    "--add-metadata",
                    "--user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
                    "--referer", "https://www.youtube.com/",
                    "--sleep-interval", "1",
                    "--max-sleep-interval", "3",
                    "-o", output_path,
                    url
                ]
            else:  # mp4
                cmd = [
                    "yt-dlp",
                    "--cookies", cookie_file,
                    "--format", f"best[height<={quality}]",
                    "--user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
                    "--referer", "https://www.youtube.com/",
                    "--sleep-interval", "1",
                    "--max-sleep-interval", "3",
                    "-o", output_path,
                    url
                ]
            
            # Execute download
            try:
                result = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
            except subprocess.TimeoutExpired:
                return {"error": "Download timeout (5 minutes)"}
            except FileNotFoundError:
                return {"error": "yt-dlp not found"}
            
            # Clean up cookie file safely
            if cookie_file:
                try:
                    os.unlink(cookie_file)
                except OSError:
                    pass  # File might already be deleted
            
            if result.returncode == 0:
                return {
                    "success": True,
                    "video_info": video_info,
                    "output_path": output_path
                }
            else:
                return {
                    "error": f"Download failed: {result.stderr}",
                    "video_info": video_info
                }
                
        except WebDriverException as e:
            return {"error": f"WebDriver error during download: {str(e)}"}
        except Exception as e:
            return {"error": f"Download failed: {str(e)}"}
    
    def close(self):
        """Clean up resources"""
        # Stop rotation service first
        if self.rotation_active:
            self.stop_rotation_service()
            
        if self.driver:
            try:
                self.driver.quit()
            except WebDriverException:
                pass  # Driver might already be closed
            finally:
                self.driver = None

    def __del__(self):
        """Ensure cleanup on object destruction"""
        self.close()

def main():
    """Main function with proper error handling"""
    parser = argparse.ArgumentParser(description='Selenium-based YouTube extractor with advanced anti-detection')
    parser.add_argument('--url', required=True, help='YouTube URL')
    parser.add_argument('--action', choices=['info', 'download'], default='info', help='Action to perform')
    parser.add_argument('--output', help='Output path for download')
    parser.add_argument('--format', choices=['mp3', 'mp4'], default='mp3', help='Output format')
    parser.add_argument('--quality', default='192', help='Quality (bitrate for mp3, resolution for mp4)')
    parser.add_argument('--headless', action='store_true', help='Run in headless mode')
    parser.add_argument('--advanced-evasion', action='store_true', help='Enable advanced bot detection evasion')
    parser.add_argument('--continuous-rotation', action='store_true', help='Enable continuous parameter rotation')
    parser.add_argument('--anti-detection', action='store_true', help='Enable full anti-detection system')
    
    args = parser.parse_args()
    
    # Enable anti-detection if any advanced features are requested
    use_anti_detection = args.advanced_evasion or args.continuous_rotation or args.anti_detection
    
    extractor = None
    try:
        extractor = YouTubeSeleniumExtractor(
            headless=args.headless,
            use_anti_detection=use_anti_detection
        )
        
        if args.action == 'info':
            if args.advanced_evasion:
                # Use async method for advanced evasion
                import asyncio
                loop = asyncio.new_event_loop()
                asyncio.set_event_loop(loop)
                result = loop.run_until_complete(extractor.get_video_info_async(args.url))
                loop.close()
            else:
                result = extractor.get_video_info(args.url)
            print(json.dumps(result, indent=2))
            
        elif args.action == 'download':
            if not args.output:
                print(json.dumps({"error": "Output path required for download"}))
                sys.exit(1)
                
            if args.advanced_evasion:
                # Use async method for advanced evasion
                import asyncio
                loop = asyncio.new_event_loop()
                asyncio.set_event_loop(loop)
                result = loop.run_until_complete(
                    extractor.download_video_async(args.url, args.output, args.format, args.quality)
                )
                loop.close()
            else:
                result = extractor.download_video(args.url, args.output, args.format, args.quality)
            print(json.dumps(result, indent=2))
            
    except KeyboardInterrupt:
        print(json.dumps({"error": "Operation cancelled by user"}), file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(json.dumps({"error": f"Unexpected error: {str(e)}"}), file=sys.stderr)
        sys.exit(1)
    finally:
        if extractor:
            extractor.close()

if __name__ == "__main__":
    main()
